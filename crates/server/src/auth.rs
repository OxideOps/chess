//! Users and sessions.
//!
//! Guests are users without a username or password; they exist so anyone
//! can play without signing up, and can be upgraded in place. Sessions are
//! server-side rows; the cookie holds only the random session id.

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{ConnectInfo, FromRef, FromRequestParts, State},
    http::{HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    db::Db,
    limit::{Limit, Limiter},
};

pub const SESSION_COOKIE: &str = "session";
const SESSION_DAYS: i64 = 30;

// Rate limits. Password guessing is bounded per username (so one address
// can't work through a list) and per address (so one address can't work
// through many usernames); signups and guests per address bound account
// spam. Generous enough that a person never sees them.
pub(crate) const LOGIN_PER_USER: Limit = Limit::per_minute(10);
pub(crate) const LOGIN_PER_ADDR: Limit = Limit::per_minute(30);
const SIGNUP_PER_ADDR: Limit = Limit::per_minute(10);
const GUEST_PER_ADDR: Limit = Limit::per_minute(30);

/// How often the background sweep runs.
const CLEANUP_EVERY: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct User {
    pub id: String,
    /// `None` for guests.
    pub username: Option<String>,
    pub is_guest: bool,
}

/// Account operations against the database.
#[derive(Clone)]
pub struct Auth {
    db: Db,
    /// Mark the cookie `Secure` (only over https). Off for plain-http development.
    secure_cookies: bool,
    limiter: Arc<Limiter>,
    /// Multiplies every limit above; 1 in production (see
    /// [`crate::Config::rate_limit_scale`]).
    rate_limit_scale: u32,
}

#[derive(Debug)]
pub enum AuthError {
    InvalidUsername,
    UsernameTaken,
    WeakPassword,
    InvalidCredentials,
    NotSignedIn,
    /// The provider identity being connected already belongs to another user.
    IdentityTaken,
    /// Removing this sign-in method would leave the account no way in.
    LastWayIn,
    /// No such sign-in method on this account.
    NoSuchIdentity,
    /// Changing a password needs the current one, and this wasn't it.
    WrongPassword,
    /// Planting a new way into the account (a first password, another
    /// provider) needs a session from a recent sign-in; this one is older.
    /// Carries the provider to sign in again with, if any, and what for.
    SignInAgain {
        provider: Option<crate::oauth::ProviderInfo>,
        /// "set a password": finishes "sign in again with Lichess to …".
        to: &'static str,
    },
    /// Guests have no account settings; they sign up first.
    GuestAccount,
    /// Rate limited; try again after this long.
    TooManyAttempts(Duration),
    /// Accounts need a database and the server was started without one.
    Unavailable,
    Db(sqlx::Error),
}

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        AuthError::Db(e)
    }
}

impl AuthError {
    /// The status and the message a response carries.
    pub(crate) fn describe(&self) -> (StatusCode, String) {
        match self {
            AuthError::InvalidUsername => (
                StatusCode::BAD_REQUEST,
                "usernames are 3-20 letters, digits or underscores".to_string(),
            ),
            AuthError::UsernameTaken => (StatusCode::CONFLICT, "that username is taken".into()),
            AuthError::WeakPassword => (
                StatusCode::BAD_REQUEST,
                "passwords are 8-128 characters".into(),
            ),
            AuthError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "wrong username or password".into(),
            ),
            AuthError::NotSignedIn => (StatusCode::UNAUTHORIZED, "not signed in".into()),
            AuthError::IdentityTaken => (
                StatusCode::CONFLICT,
                "that account already signs in another player here".into(),
            ),
            AuthError::LastWayIn => (
                StatusCode::CONFLICT,
                "this is the only way into your account; set a password or connect another \
                 sign-in method first"
                    .into(),
            ),
            AuthError::NoSuchIdentity => (
                StatusCode::NOT_FOUND,
                "that sign-in method is not connected to your account".into(),
            ),
            AuthError::WrongPassword => (
                StatusCode::FORBIDDEN,
                "your current password is not that".into(),
            ),
            AuthError::SignInAgain { provider, to } => (
                StatusCode::FORBIDDEN,
                match provider {
                    Some(p) => format!("sign in again with {} to {to}", p.name),
                    None => format!("sign in again to {to}"),
                },
            ),
            AuthError::GuestAccount => (
                StatusCode::FORBIDDEN,
                "guests have no account settings; sign up first".into(),
            ),
            AuthError::TooManyAttempts(wait) => (
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "too many attempts; try again in {} s",
                    wait.as_secs().max(1)
                ),
            ),
            AuthError::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "accounts are not available on this server".into(),
            ),
            AuthError::Db(e) => {
                tracing::error!("auth: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".into())
            }
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = self.describe();
        let mut body = serde_json::json!({ "error": message });
        // The page offers the way back: the provider flow, ending on /account.
        if let AuthError::SignInAgain {
            provider: Some(p), ..
        } = &self
        {
            body["sign_in_again"] = serde_json::json!(p);
        }
        let mut response = (status, Json(body)).into_response();
        if let AuthError::TooManyAttempts(wait) = self {
            response.headers_mut().insert(
                header::RETRY_AFTER,
                HeaderValue::from(wait.as_secs().max(1)),
            );
        }
        response
    }
}

pub fn valid_username(name: &str) -> bool {
    (3..=20).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn valid_password(password: &str) -> bool {
    (8..=128).contains(&password.len())
}

fn new_id() -> String {
    // 256 random bits, hex. Used for session ids; user ids are UUIDs.
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS randomness");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) async fn hash_password(password: String) -> String {
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString},
    };
    tokio::task::spawn_blocking(move || {
        let mut salt = [0u8; 16];
        getrandom::fill(&mut salt).expect("OS randomness");
        let salt = SaltString::encode_b64(&salt).expect("16 bytes fit a salt");
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("argon2 with default params")
            .to_string()
    })
    .await
    .expect("hashing task")
}

pub(crate) async fn verify_password(password: String, hash: String) -> bool {
    use argon2::{
        Argon2,
        password_hash::{PasswordHash, PasswordVerifier},
    };
    tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash)
            .map(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
            .unwrap_or(false)
    })
    .await
    .expect("verify task")
}

impl Auth {
    pub fn new(db: Db, secure_cookies: bool) -> Auth {
        Auth {
            db,
            secure_cookies,
            limiter: Arc::default(),
            rate_limit_scale: 1,
        }
    }

    /// Allow `factor` times the normal rate limits. For the end-to-end
    /// suite, whose many signups all come from one address.
    pub fn rate_limit_scale(mut self, factor: u32) -> Auth {
        self.rate_limit_scale = factor.max(1);
        self
    }

    pub(crate) fn db(&self) -> &Db {
        &self.db
    }

    /// One hit against `limit` for `key`; `TooManyAttempts` when over.
    pub(crate) fn limit(&self, key: &str, limit: Limit) -> Result<(), AuthError> {
        self.limiter
            .hit(key, limit.scaled(self.rate_limit_scale), Instant::now())
            .map_err(AuthError::TooManyAttempts)
    }

    /// Delete sessions past their expiry, and guests who have no session
    /// and no game (created for a visit that never played) once they are a
    /// day old. Returns `(sessions, guests)` removed.
    pub async fn purge_expired(&self) -> Result<(u64, u64), AuthError> {
        let sessions = sqlx::query!("DELETE FROM sessions WHERE expires_at < now()")
            .execute(self.db.pool())
            .await?
            .rows_affected();
        let guests = sqlx::query!(
            "DELETE FROM users u
             WHERE u.is_guest
               AND u.created_at < now() - interval '1 day'
               AND NOT EXISTS (SELECT 1 FROM sessions s WHERE s.user_id = u.id)
               AND NOT EXISTS (SELECT 1 FROM games g
                               WHERE g.white_user_id = u.id OR g.black_user_id = u.id)"
        )
        .execute(self.db.pool())
        .await?
        .rows_affected();
        Ok((sessions, guests))
    }

    /// A brand-new guest, signed in.
    pub async fn create_guest(&self) -> Result<(User, String), AuthError> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query!("INSERT INTO users (id, is_guest) VALUES ($1, true)", id)
            .execute(self.db.pool())
            .await?;
        let user = User {
            id,
            username: None,
            is_guest: true,
        };
        let session = self.create_session(&user.id).await?;
        Ok((user, session))
    }

    /// Register. A signed-in guest is upgraded in place (same id, same
    /// games); anyone else gets a new user. Returns the user and a session
    /// (the caller's own if upgrading).
    pub async fn signup(
        &self,
        username: &str,
        password: &str,
        current: Option<(&User, &str)>,
    ) -> Result<(User, String), AuthError> {
        if !valid_username(username) {
            return Err(AuthError::InvalidUsername);
        }
        if !valid_password(password) {
            return Err(AuthError::WeakPassword);
        }
        let hash = hash_password(password.to_string()).await;
        match current {
            Some((guest, session)) if guest.is_guest => {
                let updated = sqlx::query!(
                    "UPDATE users SET username = $2, password_hash = $3, is_guest = false
                     WHERE id = $1 AND is_guest",
                    guest.id,
                    username,
                    hash
                )
                .execute(self.db.pool())
                .await
                .map_err(unique_to_taken)?;
                if updated.rows_affected() == 0 {
                    // Raced with another upgrade of the same guest.
                    return Err(AuthError::NotSignedIn);
                }
                Ok((
                    User {
                        id: guest.id.clone(),
                        username: Some(username.to_string()),
                        is_guest: false,
                    },
                    session.to_string(),
                ))
            }
            _ => {
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query!(
                    "INSERT INTO users (id, username, password_hash, is_guest) VALUES ($1, $2, $3, false)",
                    id,
                    username,
                    hash
                )
                .execute(self.db.pool())
                .await
                .map_err(unique_to_taken)?;
                let user = User {
                    id,
                    username: Some(username.to_string()),
                    is_guest: false,
                };
                let session = self.create_session(&user.id).await?;
                Ok((user, session))
            }
        }
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<(User, String), AuthError> {
        let row = sqlx::query!(
            "SELECT id, username, password_hash FROM users WHERE lower(username) = lower($1)",
            username
        )
        .fetch_optional(self.db.pool())
        .await?;
        // Verify even when the user doesn't exist, so timing doesn't reveal usernames.
        let (id, name, hash) = match row {
            Some(r) => (r.id, r.username, r.password_hash),
            None => (String::new(), None, None),
        };
        let ok = verify_password(
            password.to_string(),
            hash.unwrap_or_else(|| DUMMY_HASH.to_string()),
        )
        .await
            && !id.is_empty();
        if !ok {
            return Err(AuthError::InvalidCredentials);
        }
        let user = User {
            id,
            username: name,
            is_guest: false,
        };
        let session = self.create_session(&user.id).await?;
        Ok((user, session))
    }

    /// Whether `session` comes from a sign-in in the last `minutes` (a
    /// session's `created_at` is its sign-in; sliding the expiry leaves it).
    pub(crate) async fn signed_in_within(
        &self,
        session: Option<&str>,
        minutes: i32,
    ) -> Result<bool, AuthError> {
        let Some(session) = session else {
            return Ok(false);
        };
        Ok(sqlx::query_scalar!(
            r#"SELECT created_at > now() - make_interval(mins => $2) AS "fresh!"
               FROM sessions WHERE id = $1"#,
            session,
            minutes
        )
        .fetch_optional(self.db.pool())
        .await?
        .unwrap_or(false))
    }

    pub async fn logout(&self, session: &str) -> Result<(), AuthError> {
        sqlx::query!("DELETE FROM sessions WHERE id = $1", session)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// The user behind a session id, if it exists and hasn't expired.
    /// Extends the session while it is in use.
    pub async fn user_for_session(&self, session: &str) -> Result<Option<User>, AuthError> {
        let row = sqlx::query!(
            "SELECT u.id, u.username, u.is_guest FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.id = $1 AND s.expires_at > now()",
            session
        )
        .fetch_optional(self.db.pool())
        .await?;
        let Some(row) = row else { return Ok(None) };
        // Slide the expiry, but not on every request.
        sqlx::query!(
            "UPDATE sessions SET last_seen_at = now(), expires_at = now() + make_interval(days => $2)
             WHERE id = $1 AND last_seen_at < now() - interval '5 minutes'",
            session,
            SESSION_DAYS as i32
        )
        .execute(self.db.pool())
        .await?;
        Ok(Some(User {
            id: row.id,
            username: row.username,
            is_guest: row.is_guest,
        }))
    }

    pub(crate) async fn create_session(&self, user_id: &str) -> Result<String, AuthError> {
        let id = new_id();
        sqlx::query!(
            "INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, now() + make_interval(days => $3))",
            id,
            user_id,
            SESSION_DAYS as i32
        )
        .execute(self.db.pool())
        .await?;
        Ok(id)
    }

    pub(crate) fn cookie(&self, session: String) -> Cookie<'static> {
        Cookie::build((SESSION_COOKIE, session))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(self.secure_cookies)
            .max_age(time::Duration::days(SESSION_DAYS))
            .build()
    }

    fn removal_cookie() -> Cookie<'static> {
        Cookie::build(SESSION_COOKIE).path("/").build()
    }
}

/// Run [`Auth::purge_expired`] now and then every hour, for as long as the
/// server runs.
pub fn spawn_cleanup(auth: Auth) {
    tokio::spawn(async move {
        loop {
            match auth.purge_expired().await {
                Ok((0, 0)) => {}
                Ok((sessions, guests)) => {
                    tracing::info!(
                        "cleanup: removed {sessions} expired sessions, {guests} idle guests"
                    )
                }
                Err(e) => tracing::error!("cleanup: {e:?}"),
            }
            tokio::time::sleep(CLEANUP_EVERY).await;
        }
    });
}

/// A valid argon2 hash of nothing in particular, verified against when the
/// username doesn't exist so both paths cost the same.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$Q0MaFhcx9wWTdyeoCxyvXVtQlq3AjVxx58W4RIYpPhk";

pub(crate) fn unique_to_taken(e: sqlx::Error) -> AuthError {
    match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => AuthError::UsernameTaken,
        _ => AuthError::Db(e),
    }
}

// ----- extractors -------------------------------------------------------

/// The signed-in user, if any. Never fails: no cookie, unknown session, or
/// no database all mean `None`.
pub struct CurrentUser(pub Option<User>);

/// The signed-in user; responds 401 when there isn't one.
pub struct RequireUser(pub User);

/// The session id from the cookie, when present.
pub struct SessionId(pub Option<String>);

/// Where the request came from, for rate limiting: the peer address, or
/// with `Config::trust_proxy` the rightmost `X-Forwarded-For` entry (the one
/// the trusted proxy appended). `None` when neither is available (tests
/// calling the router directly), which shares one bucket.
pub struct ClientIp(pub Option<IpAddr>);

impl ClientIp {
    pub(crate) fn key(&self) -> String {
        match self.0 {
            Some(ip) => ip.to_string(),
            None => "unknown".to_string(),
        }
    }
}

impl<S> FromRequestParts<S> for ClientIp
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if AppState::from_ref(state).config.trust_proxy
            && let Some(forwarded) = parts
                .headers
                .get_all("x-forwarded-for")
                .iter()
                .next_back()
                .and_then(|v| v.to_str().ok())
            && let Some(ip) = forwarded
                .rsplit(',')
                .next()
                .and_then(|s| s.trim().parse::<IpAddr>().ok())
        {
            return Ok(ClientIp(Some(ip)));
        }
        Ok(ClientIp(
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|c| c.0.ip()),
        ))
    }
}

impl<S> FromRequestParts<S> for SessionId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state).await?;
        Ok(SessionId(
            jar.get(SESSION_COOKIE).map(|c| c.value().to_string()),
        ))
    }
}

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let SessionId(session) = SessionId::from_request_parts(parts, state).await?;
        let state = AppState::from_ref(state);
        let user = match (session, &state.auth) {
            (Some(session), Some(auth)) => {
                auth.user_for_session(&session).await.unwrap_or_else(|e| {
                    tracing::error!("session lookup: {e:?}");
                    None
                })
            }
            _ => None,
        };
        Ok(CurrentUser(user))
    }
}

impl<S> FromRequestParts<S> for RequireUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match CurrentUser::from_request_parts(parts, state).await {
            Ok(CurrentUser(Some(user))) => Ok(RequireUser(user)),
            _ => Err(AuthError::NotSignedIn),
        }
    }
}

// ----- endpoints --------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/guest", post(guest))
        .route("/api/auth/signup", post(signup))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/me", get(me))
}

fn auth(state: &AppState) -> Result<&Auth, AuthError> {
    state.auth.as_ref().ok_or(AuthError::Unavailable)
}

async fn guest(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
    ip: ClientIp,
    jar: CookieJar,
) -> Result<(CookieJar, Json<User>), AuthError> {
    let auth = auth(&state)?;
    if let Some(user) = current {
        return Ok((jar, Json(user)));
    }
    auth.limit(&format!("guest:{}", ip.key()), GUEST_PER_ADDR)?;
    let (user, session) = auth.create_guest().await?;
    Ok((jar.add(auth.cookie(session)), Json(user)))
}

async fn signup(
    State(state): State<AppState>,
    CurrentUser(current): CurrentUser,
    SessionId(session): SessionId,
    ip: ClientIp,
    jar: CookieJar,
    Json(creds): Json<Credentials>,
) -> Result<(StatusCode, CookieJar, Json<User>), AuthError> {
    let auth = auth(&state)?;
    auth.limit(&format!("signup:{}", ip.key()), SIGNUP_PER_ADDR)?;
    let current = match (&current, &session) {
        (Some(user), Some(session)) => Some((user, session.as_str())),
        _ => None,
    };
    let (user, session) = auth
        .signup(&creds.username, &creds.password, current)
        .await?;
    Ok((
        StatusCode::CREATED,
        jar.add(auth.cookie(session)),
        Json(user),
    ))
}

async fn login(
    State(state): State<AppState>,
    ip: ClientIp,
    jar: CookieJar,
    Json(creds): Json<Credentials>,
) -> Result<(CookieJar, Json<User>), AuthError> {
    let auth = auth(&state)?;
    auth.limit(&format!("login:{}", ip.key()), LOGIN_PER_ADDR)?;
    auth.limit(
        &format!("login:{}", creds.username.to_lowercase()),
        LOGIN_PER_USER,
    )?;
    let (user, session) = auth.login(&creds.username, &creds.password).await?;
    Ok((jar.add(auth.cookie(session)), Json(user)))
}

async fn logout(
    State(state): State<AppState>,
    SessionId(session): SessionId,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), AuthError> {
    let auth = auth(&state)?;
    if let Some(session) = session {
        auth.logout(&session).await?;
    }
    Ok((jar.remove(Auth::removal_cookie()), StatusCode::NO_CONTENT))
}

async fn me(RequireUser(user): RequireUser) -> Json<User> {
    Json(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_and_password_rules() {
        assert!(valid_username("dan"));
        assert!(valid_username("Dillon_18"));
        assert!(!valid_username("ab"));
        assert!(!valid_username("has space"));
        assert!(!valid_username("émile"));
        assert!(!valid_username(&"x".repeat(21)));
        assert!(valid_password("12345678"));
        assert!(!valid_password("1234567"));
        assert!(!valid_password(&"p".repeat(129)));
    }

    #[test]
    fn session_ids_are_long_and_unique() {
        let a = new_id();
        let b = new_id();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn passwords_round_trip_and_the_dummy_hash_parses() {
        let hash = hash_password("correct horse".into()).await;
        assert!(verify_password("correct horse".into(), hash.clone()).await);
        assert!(!verify_password("wrong".into(), hash).await);
        assert!(!verify_password("anything".into(), DUMMY_HASH.into()).await);
    }
}
