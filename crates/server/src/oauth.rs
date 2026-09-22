//! Sign in with an OAuth provider (Lichess, Google).
//!
//! `GET /api/auth/{provider}/start` sends the browser to the provider with
//! a random `state` and a PKCE challenge, both remembered in a short-lived
//! cookie. `GET /api/auth/{provider}/callback` swaps the code for a token,
//! asks the provider who this is, and finds or creates the matching user:
//! `identities (provider, subject)` maps onto `users`. A signed-in guest is
//! upgraded in place, like signup; a signed-in account gets the identity
//! linked (this is "Connect" on the account page, so its failures go back
//! there, and an identity that already belongs to someone else is refused
//! rather than switching accounts); anyone else becomes a new user named
//! after their provider account (with a suffix if the name is taken). The
//! provider's label for the account is kept with the identity and refreshed
//! at each sign-in, for the account page. A built-in fake provider
//! ([`Provider::fake`]) serves development and the tests without any
//! network: it signs in as whatever name is typed.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::{
    AppState,
    auth::{Auth, AuthError, CurrentUser, SessionId, User, unique_to_taken},
};

/// The cookie that carries a flow from `start` to `callback`.
const FLOW_COOKIE: &str = "oauth";
const FLOW_MINUTES: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Lichess,
    Google,
    /// In-process, for development and tests. Never enable in production.
    Fake,
}

#[derive(Debug, Clone)]
pub struct Provider {
    pub kind: ProviderKind,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub identity_url: String,
}

/// Who the provider says the user is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The provider's stable id for this account.
    pub subject: String,
    /// What to call them, before it is made into a valid username.
    pub name: String,
    /// How the account page shows this identity: the Lichess username, the
    /// Google email. Empty when the provider gave nothing better.
    pub label: String,
}

impl Provider {
    /// Lichess is a public client: PKCE, no secret, any string as client id,
    /// and `/api/account` needs no scope.
    pub fn lichess(client_id: String) -> Provider {
        Provider {
            kind: ProviderKind::Lichess,
            client_id,
            client_secret: None,
            auth_url: "https://lichess.org/oauth".into(),
            token_url: "https://lichess.org/api/token".into(),
            identity_url: "https://lichess.org/api/account".into(),
        }
    }

    pub fn google(client_id: String, client_secret: String) -> Provider {
        Provider {
            kind: ProviderKind::Google,
            client_id,
            client_secret: Some(client_secret),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
            identity_url: "https://openidconnect.googleapis.com/v1/userinfo".into(),
        }
    }

    /// The fake provider lives inside this server (see [`fake_authorize`]);
    /// its token and identity steps are function calls, not requests.
    pub fn fake() -> Provider {
        Provider {
            kind: ProviderKind::Fake,
            client_id: "fake".into(),
            client_secret: None,
            auth_url: "/api/auth/fake-provider/authorize".into(),
            token_url: String::new(),
            identity_url: String::new(),
        }
    }

    /// The path segment and the `identities.provider` value.
    pub fn id(&self) -> &'static str {
        match self.kind {
            ProviderKind::Lichess => "lichess",
            ProviderKind::Google => "google",
            ProviderKind::Fake => "fake",
        }
    }

    pub fn name(&self) -> &'static str {
        match self.kind {
            ProviderKind::Lichess => "Lichess",
            ProviderKind::Google => "Google",
            ProviderKind::Fake => "Fake provider",
        }
    }

    fn scope(&self) -> Option<&'static str> {
        match self.kind {
            ProviderKind::Lichess | ProviderKind::Fake => None,
            ProviderKind::Google => Some("openid email profile"),
        }
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: self.id().to_string(),
            name: self.name().to_string(),
        }
    }

    /// The URL to send the browser to.
    pub fn authorize_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: &str,
    ) -> Result<String, url::ParseError> {
        let mut params = vec![
            ("response_type", "code"),
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("state", state),
            ("code_challenge", code_challenge),
            ("code_challenge_method", "S256"),
        ];
        if let Some(scope) = self.scope() {
            params.push(("scope", scope));
        }
        // A relative `auth_url` (the fake provider) keeps its own origin.
        let base = url::Url::parse("http://relative.invalid/")?;
        let mut url = base.join(&self.auth_url)?;
        url.query_pairs_mut().extend_pairs(params);
        Ok(if self.auth_url.starts_with('/') {
            url[url::Position::BeforePath..].to_string()
        } else {
            url.to_string()
        })
    }

    /// Turn the callback's code into who the user is.
    async fn exchange(
        &self,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
    ) -> Result<Identity, String> {
        if self.kind == ProviderKind::Fake {
            return fake_exchange(code, verifier);
        }
        #[derive(Deserialize)]
        struct Token {
            access_token: String,
        }
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", self.client_id.as_str()),
            ("code_verifier", verifier),
        ];
        if let Some(secret) = &self.client_secret {
            form.push(("client_secret", secret));
        }
        let client = reqwest::Client::new();
        let token: Token = client
            .post(&self.token_url)
            .form(&form)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("token request: {e}"))?
            .json()
            .await
            .map_err(|e| format!("token response: {e}"))?;
        let who: serde_json::Value = client
            .get(&self.identity_url)
            .bearer_auth(token.access_token)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("identity request: {e}"))?
            .json()
            .await
            .map_err(|e| format!("identity response: {e}"))?;
        self.parse_identity(&who)
            .ok_or_else(|| "identity response is missing fields".to_string())
    }

    fn parse_identity(&self, who: &serde_json::Value) -> Option<Identity> {
        let text = |key: &str| who.get(key).and_then(|v| v.as_str()).map(str::to_string);
        match self.kind {
            ProviderKind::Lichess => {
                let name = text("username")?;
                Some(Identity {
                    subject: text("id")?,
                    label: name.clone(),
                    name,
                })
            }
            ProviderKind::Google => {
                let subject = text("sub")?;
                let email = text("email");
                let name = email
                    .as_deref()
                    .and_then(|e| e.split('@').next().map(str::to_string))
                    .or_else(|| text("name"))
                    .unwrap_or_default();
                let label = email.or_else(|| text("name")).unwrap_or_default();
                Some(Identity {
                    subject,
                    name,
                    label,
                })
            }
            ProviderKind::Fake => None,
        }
    }
}

/// A provider users can pick on the login page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
}

// ----- PKCE and names ---------------------------------------------------

fn random_urlsafe(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("OS randomness");
    URL_SAFE_NO_PAD.encode(buf)
}

/// `S256`: the challenge is the base64url SHA-256 of the verifier.
pub fn code_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// A valid username from whatever the provider called the user: the
/// allowed characters, at most 20, at least 3.
pub fn derive_username(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(20)
        .collect();
    if kept.len() < 3 {
        format!("user{kept}")
    } else {
        kept
    }
}

/// `alice_4821` when `alice` is taken.
fn with_suffix(base: &str) -> String {
    let mut n = [0u8; 2];
    getrandom::fill(&mut n).expect("OS randomness");
    let digits = u16::from_le_bytes(n) % 10_000;
    let base: String = base.chars().take(15).collect();
    format!("{base}_{digits:04}")
}

/// Only paths on this site, so a crafted link can't send a freshly
/// signed-in user elsewhere.
fn safe_next(next: Option<&str>) -> String {
    match next {
        Some(n) if n.starts_with('/') && !n.starts_with("//") && !n.starts_with("/\\") => {
            n.to_string()
        }
        _ => "/".to_string(),
    }
}

// ----- the flow ---------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct Flow {
    provider: String,
    state: String,
    verifier: String,
    next: String,
}

impl Flow {
    fn to_cookie(&self, secure: bool) -> Cookie<'static> {
        let value = URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).expect("serializable"));
        Cookie::build((FLOW_COOKIE, value))
            .path("/api/auth")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(secure)
            .max_age(time::Duration::minutes(FLOW_MINUTES))
            .build()
    }

    fn from_cookie(jar: &CookieJar) -> Option<Flow> {
        let bytes = URL_SAFE_NO_PAD.decode(jar.get(FLOW_COOKIE)?.value()).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    fn removal() -> Cookie<'static> {
        Cookie::build(FLOW_COOKIE).path("/api/auth").build()
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/providers", get(providers))
        .route("/api/auth/{provider}/start", get(start))
        .route("/api/auth/{provider}/callback", get(callback))
        .route("/api/auth/fake-provider/authorize", get(fake_authorize))
}

async fn providers(State(state): State<AppState>) -> Json<Vec<ProviderInfo>> {
    Json(state.config.oauth.iter().map(Provider::info).collect())
}

fn provider<'a>(state: &'a AppState, id: &str) -> Result<&'a Provider, StatusCode> {
    state
        .config
        .oauth
        .iter()
        .find(|p| p.id() == id)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Where the provider sends the browser back to: the configured public
/// URL, else this request's own host.
fn redirect_uri(state: &AppState, headers: &HeaderMap, provider: &Provider) -> String {
    let origin = match &state.config.public_url {
        Some(url) => url.trim_end_matches('/').to_string(),
        None => {
            let host = headers
                .get(header::HOST)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost");
            let scheme = if state.config.secure_cookies {
                "https"
            } else {
                "http"
            };
            format!("{scheme}://{host}")
        }
    };
    format!("{origin}/api/auth/{}/callback", provider.id())
}

#[derive(Deserialize)]
struct StartQuery {
    next: Option<String>,
}

async fn start(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<StartQuery>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), StatusCode> {
    if state.auth.is_none() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let provider = provider(&state, &id)?;
    let flow = Flow {
        provider: provider.id().to_string(),
        state: random_urlsafe(16),
        verifier: random_urlsafe(32),
        next: safe_next(query.next.as_deref()),
    };
    let url = provider
        .authorize_url(
            &redirect_uri(&state, &headers, provider),
            &flow.state,
            &code_challenge(&flow.verifier),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        jar.add(flow.to_cookie(state.config.secure_cookies)),
        Redirect::to(&url),
    ))
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    /// Set by the provider when the user cancelled or something failed.
    error: Option<String>,
}

async fn callback(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<CallbackQuery>,
    CurrentUser(current): CurrentUser,
    SessionId(session): SessionId,
    headers: HeaderMap,
    jar: CookieJar,
) -> Response {
    // The flow cookie is single-use: read it, then drop it either way.
    let flow = Flow::from_cookie(&jar);
    let jar = jar.remove(Flow::removal());
    let Ok(provider) = provider(&state, &id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(auth) = &state.auth else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let next = flow
        .as_ref()
        .map_or_else(|| "/".to_string(), |f| f.next.clone());
    let current = match (&current, &session) {
        (Some(user), Some(session)) => Some((user, session.as_str())),
        _ => None,
    };
    // For a signed-in account this flow links rather than signs in: it is
    // connecting another method, which is done (and its failures shown) on
    // the account page, not the login page.
    let connecting = current.is_some_and(|(user, _)| !user.is_guest);
    match finish(&state, provider, auth, flow, query, current, &headers).await {
        Ok((user, session)) => {
            tracing::info!(
                "signed in via {}: {}",
                provider.id(),
                user.username.as_deref().unwrap_or("?")
            );
            (jar.add(auth.cookie(session)), Redirect::to(&next)).into_response()
        }
        Err(message) => {
            tracing::warn!("{} sign-in failed: {message}", provider.id());
            let text = if connecting {
                format!("Connecting {} failed: {message}", provider.name())
            } else {
                format!("Sign-in with {} failed: {message}", provider.name())
            };
            let page = if connecting { "/account" } else { "/login" };
            let mut url = url::Url::parse("http://relative.invalid/")
                .and_then(|base| base.join(page))
                .expect("static");
            url.query_pairs_mut().append_pair("error", &text);
            if !connecting {
                url.query_pairs_mut().append_pair("next", &next);
            }
            let location = url[url::Position::BeforePath..].to_string();
            (jar, Redirect::to(&location)).into_response()
        }
    }
}

/// Everything that can go wrong, as a message for the login page.
async fn finish(
    state: &AppState,
    provider: &Provider,
    auth: &Auth,
    flow: Option<Flow>,
    query: CallbackQuery,
    current: Option<(&User, &str)>,
    headers: &HeaderMap,
) -> Result<(User, String), String> {
    if let Some(error) = query.error {
        return Err(if error == "access_denied" {
            "you cancelled".to_string()
        } else {
            format!("the provider said {error}")
        });
    }
    let flow = flow.ok_or("the sign-in took too long or the cookie was lost; try again")?;
    if flow.provider != provider.id() || query.state.as_deref() != Some(flow.state.as_str()) {
        return Err("the reply did not match the request; try again".to_string());
    }
    let code = query.code.ok_or("no code came back")?;
    let identity = provider
        .exchange(
            &code,
            &flow.verifier,
            &redirect_uri(state, headers, provider),
        )
        .await?;
    sign_in(auth, provider.id(), &identity, current)
        .await
        .map_err(|e| match e {
            AuthError::IdentityTaken => format!(
                "that {} account already signs in another player here",
                provider.name()
            ),
            e => {
                tracing::error!("oauth sign-in: {e:?}");
                "database error".to_string()
            }
        })
}

/// Find the user behind `identity`, or make one: upgrade a signed-in guest,
/// link to a signed-in account, or create a fresh user named after the
/// provider account. Returns the user and a session for them.
pub async fn sign_in(
    auth: &Auth,
    provider: &str,
    identity: &Identity,
    current: Option<(&User, &str)>,
) -> Result<(User, String), AuthError> {
    let pool = auth.db().pool();
    let label = Some(identity.label.as_str()).filter(|l| !l.is_empty());
    let known = sqlx::query!(
        "SELECT u.id, u.username, u.is_guest FROM identities i JOIN users u ON u.id = i.user_id
         WHERE i.provider = $1 AND i.subject = $2",
        provider,
        identity.subject
    )
    .fetch_optional(pool)
    .await?;
    if let Some(row) = known {
        let user = User {
            id: row.id,
            username: row.username,
            is_guest: row.is_guest,
        };
        // Keep the label current: people rename themselves at the provider.
        sqlx::query!(
            "UPDATE identities SET label = COALESCE($3, label) WHERE provider = $1 AND subject = $2",
            provider,
            identity.subject,
            label
        )
        .execute(pool)
        .await?;
        return match current {
            // Connecting what is already connected: nothing changes.
            Some((me, session)) if me.id == user.id => Ok((user, session.to_string())),
            // A signed-in account never silently becomes someone else by
            // "connecting" an identity that belongs to another player.
            Some((me, _)) if !me.is_guest => Err(AuthError::IdentityTaken),
            _ => {
                let session = auth.create_session(&user.id).await?;
                Ok((user, session))
            }
        };
    }

    let (user, session) = match current {
        // A registered account links the identity and keeps its session.
        Some((user, session)) if !user.is_guest => (user.clone(), session.to_string()),
        // A guest is upgraded in place: same id, same games.
        Some((guest, session)) => {
            let username = claim_username(pool, &identity.name, |name| {
                let id = guest.id.clone();
                async move {
                    sqlx::query!(
                        "UPDATE users SET username = $2, is_guest = false WHERE id = $1 AND is_guest",
                        id,
                        name
                    )
                    .execute(pool)
                    .await
                    .map(|done| done.rows_affected() == 1)
                }
            })
            .await?
            .ok_or(AuthError::NotSignedIn)?; // raced with another upgrade
            (
                User {
                    id: guest.id.clone(),
                    username: Some(username),
                    is_guest: false,
                },
                session.to_string(),
            )
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            let username = claim_username(pool, &identity.name, |name| {
                let id = id.clone();
                async move {
                    sqlx::query!(
                        "INSERT INTO users (id, username, is_guest) VALUES ($1, $2, false)",
                        id,
                        name
                    )
                    .execute(pool)
                    .await
                    .map(|_| true)
                }
            })
            .await?
            .ok_or(AuthError::NotSignedIn)?; // cannot happen: an insert is never a no-op
            let session = auth.create_session(&id).await?;
            (
                User {
                    id,
                    username: Some(username),
                    is_guest: false,
                },
                session,
            )
        }
    };
    sqlx::query!(
        "INSERT INTO identities (provider, subject, user_id, label) VALUES ($1, $2, $3, $4)",
        provider,
        identity.subject,
        user.id,
        label
    )
    .execute(pool)
    .await?;
    Ok((user, session))
}

/// Run `write` with a username derived from `name`, retrying with a random
/// suffix while the name is taken. `Ok(None)` when `write` found nothing to
/// update; `Ok(Some(username))` with the name that stuck.
async fn claim_username<F, Fut>(
    _pool: &sqlx::PgPool,
    name: &str,
    write: F,
) -> Result<Option<String>, AuthError>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<bool, sqlx::Error>>,
{
    let base = derive_username(name);
    let mut candidate = base.clone();
    for _ in 0..6 {
        match write(candidate.clone()).await {
            Ok(true) => return Ok(Some(candidate)),
            Ok(false) => return Ok(None),
            Err(e) => match unique_to_taken(e) {
                AuthError::UsernameTaken => candidate = with_suffix(&base),
                other => return Err(other),
            },
        }
    }
    Err(AuthError::UsernameTaken)
}

// ----- the fake provider ------------------------------------------------

/// Codes the fake provider has issued: code → (PKCE challenge, name).
static FAKE_CODES: LazyLock<Mutex<HashMap<String, (String, String)>>> =
    LazyLock::new(Mutex::default);

#[derive(Deserialize)]
struct FakeAuthorizeQuery {
    redirect_uri: String,
    state: String,
    code_challenge: String,
    /// Who to sign in as. Without it, a form asks.
    #[serde(rename = "as")]
    name: Option<String>,
    /// Pretend the user clicked "cancel".
    deny: Option<String>,
}

/// The fake provider's "sign in" page: a name box, or with `?as=name` an
/// immediate redirect back with a code (tests use that).
async fn fake_authorize(
    State(state): State<AppState>,
    Query(q): Query<FakeAuthorizeQuery>,
) -> Response {
    if provider(&state, "fake").is_err() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut back = match url::Url::parse(&q.redirect_uri) {
        Ok(url) => url,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad redirect_uri").into_response(),
    };
    if q.deny.is_some() {
        back.query_pairs_mut()
            .append_pair("error", "access_denied")
            .append_pair("state", &q.state);
        return Redirect::to(back.as_str()).into_response();
    }
    let Some(name) = q.name.filter(|n| !n.trim().is_empty()) else {
        let escape = |s: &str| s.replace('&', "&amp;").replace('"', "&quot;");
        let page = format!(
            r#"<!doctype html><title>Fake sign-in</title>
<h1>Fake provider</h1>
<form method="get">
<input type="hidden" name="redirect_uri" value="{}">
<input type="hidden" name="state" value="{}">
<input type="hidden" name="code_challenge" value="{}">
<label>Sign in as <input name="as" value="fake_user" autofocus></label>
<button type="submit">Continue</button>
<button type="submit" name="deny" value="1">Cancel</button>
</form>"#,
            escape(&q.redirect_uri),
            escape(&q.state),
            escape(&q.code_challenge)
        );
        return Html(page).into_response();
    };
    let code = random_urlsafe(16);
    FAKE_CODES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(code.clone(), (q.code_challenge, name.trim().to_string()));
    back.query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", &q.state);
    Redirect::to(back.as_str()).into_response()
}

/// The fake provider's token + identity steps: one use per code, and the
/// PKCE verifier must match the challenge the code was issued for.
fn fake_exchange(code: &str, verifier: &str) -> Result<Identity, String> {
    let (challenge, name) = FAKE_CODES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(code)
        .ok_or("unknown or used code")?;
    if code_challenge(verifier) != challenge {
        return Err("PKCE verifier does not match".to_string());
    }
    Ok(Identity {
        subject: name.to_lowercase(),
        label: name.clone(),
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_rfc_7636_s256() {
        // The example from RFC 7636 appendix B.
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn usernames_are_derived_and_suffixed() {
        assert_eq!(derive_username("Alice"), "Alice");
        assert_eq!(derive_username("dan.albl@example.com"), "danalblexamplecom");
        assert_eq!(derive_username("a"), "usera");
        assert_eq!(derive_username(""), "user");
        assert_eq!(derive_username("x".repeat(30).as_str()).len(), 20);
        let s = with_suffix(&"y".repeat(20));
        assert_eq!(s.len(), 20, "{s}");
        assert!(s.starts_with("yyyyyyyyyyyyyyy_"), "{s}");
        assert!(crate::auth::valid_username(&s), "{s}");
    }

    #[test]
    fn authorize_urls_carry_pkce_and_keep_relative_bases() {
        let lichess = Provider::lichess("chess.example".into());
        let url = lichess
            .authorize_url(
                "https://chess.example/api/auth/lichess/callback",
                "st",
                "ch",
            )
            .unwrap();
        assert!(url.starts_with("https://lichess.org/oauth?response_type=code&client_id=chess.example&redirect_uri=https%3A%2F%2Fchess.example%2Fapi%2Fauth%2Flichess%2Fcallback&state=st&code_challenge=ch&code_challenge_method=S256"), "{url}");
        assert!(!url.contains("scope"));
        let google = Provider::google("id".into(), "secret".into());
        let url = google.authorize_url("https://x/cb", "st", "ch").unwrap();
        assert!(url.contains("&scope=openid+email+profile"), "{url}");
        let fake = Provider::fake()
            .authorize_url("http://h/cb", "st", "ch")
            .unwrap();
        assert!(
            fake.starts_with("/api/auth/fake-provider/authorize?"),
            "{fake}"
        );
    }

    #[test]
    fn identities_are_parsed_per_provider() {
        let lichess = Provider::lichess("x".into());
        let who = serde_json::json!({"id": "dan", "username": "Dan"});
        assert_eq!(
            lichess.parse_identity(&who).unwrap(),
            Identity {
                subject: "dan".into(),
                name: "Dan".into(),
                label: "Dan".into(),
            }
        );
        assert!(lichess.parse_identity(&serde_json::json!({})).is_none());
        let google = Provider::google("x".into(), "y".into());
        let who =
            serde_json::json!({"sub": "1234", "email": "dillon.r@gmail.com", "name": "Dillon"});
        assert_eq!(
            google.parse_identity(&who).unwrap(),
            Identity {
                subject: "1234".into(),
                name: "dillon.r".into(),
                label: "dillon.r@gmail.com".into(),
            }
        );
        let who = serde_json::json!({"sub": "1234", "name": "Dillon"});
        let parsed = google.parse_identity(&who).unwrap();
        assert_eq!(
            (parsed.name.as_str(), parsed.label.as_str()),
            ("Dillon", "Dillon")
        );
    }

    #[test]
    fn next_is_same_site_only() {
        assert_eq!(safe_next(Some("/game/1?x=1")), "/game/1?x=1");
        assert_eq!(safe_next(Some("//evil")), "/");
        assert_eq!(safe_next(Some("https://evil")), "/");
        assert_eq!(safe_next(None), "/");
    }

    #[test]
    fn flow_cookie_round_trips() {
        let flow = Flow {
            provider: "fake".into(),
            state: "s".into(),
            verifier: "v".into(),
            next: "/games".into(),
        };
        let cookie = flow.to_cookie(false);
        assert_eq!(cookie.path(), Some("/api/auth"));
        let jar = CookieJar::new().add(cookie);
        let back = Flow::from_cookie(&jar).unwrap();
        assert_eq!(
            (back.provider, back.state, back.verifier, back.next),
            ("fake".into(), "s".into(), "v".into(), "/games".into())
        );
    }
}
