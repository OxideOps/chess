//! An email address on an account, and resetting a forgotten password with it.
//!
//! - `GET /api/auth/mail` says whether the server can send email at all.
//! - `PUT /api/me/email` adds or changes the address: it is only *pending*
//!   until the link mailed to it is followed (`POST /api/auth/verify-email`),
//!   and a change tells the old address. `DELETE /api/me/email` removes it.
//! - `POST /api/auth/forgot-password` mails a reset link to a *verified*
//!   address; `POST /api/auth/reset-password` spends it, sets the password
//!   and signs every session of the account out.
//!
//! Links carry a random token; the database keeps only its SHA-256, in
//! `email_tokens`, and following a link deletes that row, so each link works
//! once. `users.email` holds nothing but verified addresses, so an address
//! nobody has proved can never receive a reset.
//!
//! "Forgot" answers `202` at once, whatever the address, and does the
//! lookup and the sending afterwards: the reply can't say, by its content or
//! its timing, whether the address has an account.

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AppState,
    account::{Account, load_account, registered},
    auth::{Auth, AuthError, ClientIp, RequireUser, User, hash_password, new_id, valid_password},
    limit::Limit,
    mail::{Mailer, parse_address},
};

/// How long a verification link works.
pub const VERIFY_HOURS: i32 = 24;
/// How long a reset link works.
pub const RESET_MINUTES: i32 = 60;

/// Mail sent to any one address, whatever asked for it (signup, adding it,
/// "forgot"), so the site can't be used to flood someone's inbox.
const MAIL_PER_RECIPIENT: Limit = Limit::per_hour(5);
/// "Forgot" requests from one client address.
const FORGOT_PER_ADDR: Limit = Limit::per_hour(20);
/// Address changes per account.
const CHANGES_PER_USER: Limit = Limit::per_hour(10);
/// Links followed from one client address (a token is 256 bits, so this is
/// only against noise).
const LINKS_PER_ADDR: Limit = Limit::per_minute(30);

/// `GET /api/auth/mail`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MailStatus {
    /// Whether accounts can have an address and reset a password with it.
    pub enabled: bool,
}

/// `PUT /api/me/email`, and `POST /api/auth/forgot-password`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EmailAddress {
    pub email: String,
}

/// `POST /api/auth/verify-email`: the token from the link.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EmailLink {
    pub token: String,
}

/// What `POST /api/auth/verify-email` confirmed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EmailVerified {
    pub email: String,
    pub username: String,
}

/// `POST /api/auth/reset-password`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PasswordReset {
    pub token: String,
    pub password: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/mail", get(status))
        .route("/api/me/email", put(change_email).delete(remove_email))
        .route("/api/auth/verify-email", post(verify_email))
        .route("/api/auth/forgot-password", post(forgot_password))
        .route("/api/auth/reset-password", post(reset_password))
}

fn auth(state: &AppState) -> Result<&Auth, AuthError> {
    state.auth.as_ref().ok_or(AuthError::Unavailable)
}

fn mailer(state: &AppState) -> Result<&Mailer, AuthError> {
    state.mail.as_ref().ok_or(AuthError::MailUnavailable)
}

/// What the database keeps of a token.
fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Clone, Copy)]
enum Purpose {
    Verify,
    Reset,
}

impl Purpose {
    fn as_str(self) -> &'static str {
        match self {
            Purpose::Verify => "verify",
            Purpose::Reset => "reset",
        }
    }

    fn minutes(self) -> i32 {
        match self {
            Purpose::Verify => VERIFY_HOURS * 60,
            Purpose::Reset => RESET_MINUTES,
        }
    }
}

/// A new link for `user_id`, replacing any earlier one for the same purpose
/// (only the latest verification or reset link works). Returns the token.
async fn issue(
    auth: &Auth,
    user_id: &str,
    purpose: Purpose,
    email: &str,
) -> Result<String, AuthError> {
    let token = new_id();
    let mut tx = auth.db().pool().begin().await?;
    sqlx::query!(
        "DELETE FROM email_tokens WHERE user_id = $1 AND purpose = $2",
        user_id,
        purpose.as_str()
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO email_tokens (token_hash, user_id, purpose, email, expires_at)
         VALUES ($1, $2, $3, $4, now() + make_interval(mins => $5))",
        token_hash(&token),
        user_id,
        purpose.as_str(),
        email,
        purpose.minutes()
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(token)
}

/// A token, spent: its row is deleted in `tx` and returned if it was for
/// `purpose` and hasn't expired. A second use finds nothing.
async fn spend(
    tx: &mut sqlx::PgConnection,
    token: &str,
    purpose: Purpose,
) -> Result<Option<(String, String)>, AuthError> {
    let row = sqlx::query!(
        r#"DELETE FROM email_tokens WHERE token_hash = $1 AND purpose = $2
           RETURNING user_id, email, expires_at > now() AS "live!""#,
        token_hash(token),
        purpose.as_str()
    )
    .fetch_optional(&mut *tx)
    .await?;
    Ok(row.filter(|r| r.live).map(|r| (r.user_id, r.email)))
}

/// Send in the background, logging a failure: for mail whose outcome the
/// caller mustn't wait on or reveal.
fn send_later(mailer: &Mailer, to: String, subject: String, body: String) {
    let mailer = mailer.clone();
    tokio::spawn(async move {
        if let Err(e) = mailer.send(&to, &subject, &body).await {
            tracing::error!("mail: \"{subject}\" not sent: {e}");
        }
    });
}

fn verify_message(username: &str, email: &str, origin: &str, token: &str) -> (String, String) {
    (
        "Confirm your email address".to_string(),
        format!(
            "Follow this link to confirm {email} as the address for {username} on Chess:\n\n\
             {origin}/verify-email?token={token}\n\n\
             Once it is confirmed you can reset your password with it if you ever forget it. \
             The link works for {VERIFY_HOURS} hours.\n\n\
             If you didn't ask for this, ignore it: nothing changes until the link is followed.\n"
        ),
    )
}

/// Mail a verification link for `email` to `user`'s pending address. Used
/// by signup and by `PUT /api/me/email`; the caller has checked the address.
pub(crate) async fn start_verification(
    state: &AppState,
    user: &User,
    email: &str,
    origin: &str,
) -> Result<(), AuthError> {
    let auth = auth(state)?;
    let mailer = mailer(state)?;
    auth.limit(
        &format!("mailto:{}", email.to_lowercase()),
        MAIL_PER_RECIPIENT,
    )?;
    let token = issue(auth, &user.id, Purpose::Verify, email).await?;
    let username = user.username.as_deref().unwrap_or_default();
    let (subject, body) = verify_message(username, email, origin, &token);
    mailer.send(email, &subject, &body).await.map_err(|e| {
        tracing::error!("mail: verification not sent: {e}");
        AuthError::MailFailed
    })
}

async fn status(State(state): State<AppState>) -> Json<MailStatus> {
    Json(MailStatus {
        enabled: state.mail.is_some() && state.auth.is_some(),
    })
}

async fn change_email(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
    headers: HeaderMap,
    Json(change): Json<EmailAddress>,
) -> Result<Json<Account>, AuthError> {
    let auth = registered(&state, &user)?;
    mailer(&state)?;
    auth.limit(&format!("email:{}", user.id), CHANGES_PER_USER)?;
    let email = parse_address(&change.email).ok_or(AuthError::InvalidEmail)?;
    let current = sqlx::query_scalar!("SELECT email FROM users WHERE id = $1", user.id)
        .fetch_one(auth.db().pool())
        .await?;
    if current.is_some_and(|c| c.eq_ignore_ascii_case(&email)) {
        // Already this one: a pending change to something else is dropped.
        sqlx::query!(
            "DELETE FROM email_tokens WHERE user_id = $1 AND purpose = 'verify'",
            user.id
        )
        .execute(auth.db().pool())
        .await?;
    } else {
        // Whether the address is someone else's is only found out when its
        // link is followed, by whoever reads that mailbox: nobody learns
        // here which addresses have accounts.
        let origin = state.config.public_origin(&headers);
        start_verification(&state, &user, &email, &origin).await?;
    }
    Ok(Json(load_account(&state, auth, &user).await?))
}

async fn remove_email(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
) -> Result<Json<Account>, AuthError> {
    let auth = registered(&state, &user)?;
    let mut tx = auth.db().pool().begin().await?;
    let old = sqlx::query_scalar!("SELECT email FROM users WHERE id = $1 FOR UPDATE", user.id)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query!("UPDATE users SET email = NULL WHERE id = $1", user.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM email_tokens WHERE user_id = $1", user.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    if let (Some(old), Some(mailer)) = (old, &state.mail) {
        let username = user.username.as_deref().unwrap_or_default();
        send_later(
            mailer,
            old,
            "Your email address was removed".into(),
            format!(
                "This address is no longer on {username}'s account on Chess, so a forgotten \
                 password can't be reset with it.\n\n\
                 If that wasn't you, someone else is signed in to your account: log in and \
                 change your password.\n"
            ),
        );
    }
    Ok(Json(load_account(&state, auth, &user).await?))
}

async fn verify_email(
    State(state): State<AppState>,
    ip: ClientIp,
    Json(link): Json<EmailLink>,
) -> Result<Json<EmailVerified>, AuthError> {
    let auth = auth(&state)?;
    auth.limit(&format!("link:{}", ip.key()), LINKS_PER_ADDR)?;
    let mut tx = auth.db().pool().begin().await?;
    let Some((user_id, email)) = spend(&mut tx, &link.token, Purpose::Verify).await? else {
        tx.commit().await?;
        return Err(AuthError::BadLink);
    };
    let old = sqlx::query!(
        r#"SELECT username AS "username!", email FROM users WHERE id = $1 FOR UPDATE"#,
        user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query!("UPDATE users SET email = $2 WHERE id = $1", user_id, email)
        .execute(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => AuthError::EmailTaken,
            _ => AuthError::Db(e),
        })?;
    // Reset links out for the old address die with it.
    sqlx::query!("DELETE FROM email_tokens WHERE user_id = $1", user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    tracing::info!("{} verified an email address", old.username);
    if let (Some(previous), Some(mailer)) = (old.email, &state.mail)
        && !previous.eq_ignore_ascii_case(&email)
    {
        send_later(
            mailer,
            previous,
            "Your email address was changed".into(),
            format!(
                "The address for {} on Chess is now {email}, not this one. Password resets \
                 go there from now on.\n\n\
                 If that wasn't you, someone else is signed in to your account: log in and \
                 change your password.\n",
                old.username
            ),
        );
    }
    Ok(Json(EmailVerified {
        email,
        username: old.username,
    }))
}

async fn forgot_password(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Json(request): Json<EmailAddress>,
) -> Result<StatusCode, AuthError> {
    let auth = auth(&state)?.clone();
    let mailer = mailer(&state)?.clone();
    auth.limit(&format!("forgot:{}", ip.key()), FORGOT_PER_ADDR)?;
    let email = parse_address(&request.email).ok_or(AuthError::InvalidEmail)?;
    // Counted per address asked about, account or not, so the limit says
    // nothing either.
    auth.limit(
        &format!("mailto:{}", email.to_lowercase()),
        MAIL_PER_RECIPIENT,
    )?;
    let origin = state.config.public_origin(&headers);
    tokio::spawn(async move {
        if let Err(e) = send_reset(&auth, &mailer, &email, &origin).await {
            tracing::error!("forgot password: {e:?}");
        }
    });
    Ok(StatusCode::ACCEPTED)
}

/// The part of "forgot" that happens after the reply: find the account whose
/// verified address this is, if any, and mail it a link.
async fn send_reset(
    auth: &Auth,
    mailer: &Mailer,
    email: &str,
    origin: &str,
) -> Result<(), AuthError> {
    let row = sqlx::query!(
        r#"SELECT id, username AS "username!", email AS "email!" FROM users
           WHERE lower(email) = lower($1)"#,
        email
    )
    .fetch_optional(auth.db().pool())
    .await?;
    let Some(row) = row else {
        tracing::info!("forgot password: no account has that address");
        return Ok(());
    };
    let token = issue(auth, &row.id, Purpose::Reset, &row.email).await?;
    let body = format!(
        "Someone (hopefully you) asked to reset the password for {username} on Chess. \
         Follow this link to choose a new one:\n\n\
         {origin}/reset-password?token={token}\n\n\
         The link works once, for {RESET_MINUTES} minutes. Setting a new password signs \
         {username} out everywhere.\n\n\
         If you didn't ask, ignore this: your password stays as it is.\n",
        username = row.username
    );
    mailer
        .send(&row.email, "Reset your password", &body)
        .await
        .map_err(|e| {
            tracing::error!("mail: reset not sent: {e}");
            AuthError::MailFailed
        })
}

async fn reset_password(
    State(state): State<AppState>,
    ip: ClientIp,
    Json(reset): Json<PasswordReset>,
) -> Result<StatusCode, AuthError> {
    let auth = auth(&state)?;
    auth.limit(&format!("link:{}", ip.key()), LINKS_PER_ADDR)?;
    // Checked before the link is spent, so a too-short password doesn't
    // cost the user their link.
    if !valid_password(&reset.password) {
        return Err(AuthError::WeakPassword);
    }
    let hash = hash_password(reset.password).await;
    let mut tx = auth.db().pool().begin().await?;
    let Some((user_id, email)) = spend(&mut tx, &reset.token, Purpose::Reset).await? else {
        tx.commit().await?;
        return Err(AuthError::BadLink);
    };
    // Only while the link's address is still the account's.
    let updated = sqlx::query!(
        "UPDATE users SET password_hash = $2 WHERE id = $1 AND lower(email) = lower($3)",
        user_id,
        hash,
        email
    )
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        tx.commit().await?;
        return Err(AuthError::BadLink);
    }
    // Whoever knew the old password is out, this browser included.
    sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    tracing::info!("a password was reset by email");
    Ok(StatusCode::NO_CONTENT)
}
