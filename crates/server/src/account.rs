//! The signed-in account's own settings: how it signs in.
//!
//! `GET /api/me/account` lists the connected provider identities and whether
//! there is a password; `DELETE /api/me/identities/{provider}/{subject}`
//! disconnects one, refused when it is the last way in (no password and no
//! other identity with a provider that is switched on); `PUT /api/me/password`
//! sets or changes the password. Changing it needs the current one; setting a
//! first one needs a session from a sign-in in the last
//! [`FRESH_SIGN_IN_MINUTES`], so a stolen session cookie can't plant a
//! password of its own. Either signs out every other session and cancels
//! every emailed link still out for the account.
//! Connecting a provider is the ordinary OAuth flow started while signed in
//! (see [`crate::oauth`]), which links rather than creating a user; a new
//! identity is a new way in too, so it needs the same fresh sign-in
//! ([`require_fresh_sign_in`]). The fix for a stale session is signing in
//! again: through a provider the account has (the refusal names one), or
//! with the password at the login form, which is how an account with a
//! password proves itself here (the connect flow is a redirect and can't
//! carry one).

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, put},
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    auth::{
        Auth, AuthError, ClientIp, LOGIN_PER_ADDR, LOGIN_PER_USER, RequireUser, SessionId, User,
        hash_password, valid_password, verify_password,
    },
    oauth::ProviderInfo,
};

/// How recent the sign-in behind a session must be for it to set an
/// account's first password.
pub const FRESH_SIGN_IN_MINUTES: i32 = 10;

/// What a fresh sign-in is asked for when connecting a provider.
pub(crate) const TO_CONNECT: &str = "connect another sign-in method";

/// How the signed-in account gets in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Account {
    pub username: String,
    pub has_password: bool,
    /// The verified address password resets go to; `None` means a
    /// forgotten password can't be recovered.
    pub email: Option<String>,
    /// An address added or changed to, waiting for its link to be followed.
    pub pending_email: Option<String>,
    /// Oldest first.
    pub identities: Vec<LinkedIdentity>,
}

/// One provider account connected to this one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct LinkedIdentity {
    /// The provider's id, as in `/api/auth/{provider}/start`.
    pub provider: String,
    /// What to call the provider ("Lichess").
    pub provider_name: String,
    /// The provider's id for the account; with `provider`, what to disconnect.
    pub subject: String,
    /// What the provider calls the account (a username, an email). `None`
    /// for identities connected before labels were kept, until next used.
    pub label: Option<String>,
}

/// `PUT /api/me/password`. `current` is needed when there already is one.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PasswordChange {
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    #[serde(default)]
    pub current: Option<String>,
    pub password: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/me/account", get(account))
        .route(
            "/api/me/identities/{provider}/{subject}",
            delete(disconnect),
        )
        .route("/api/me/password", put(set_password))
}

/// Accounts only: a guest has nothing here to change.
pub(crate) fn registered<'a>(state: &'a AppState, user: &User) -> Result<&'a Auth, AuthError> {
    let auth = state.auth.as_ref().ok_or(AuthError::Unavailable)?;
    if user.is_guest {
        return Err(AuthError::GuestAccount);
    }
    Ok(auth)
}

/// The provider's display name, from the configured ones, else from its id
/// (an identity outlives its provider being switched off).
fn provider_name(state: &AppState, id: &str) -> String {
    if let Some(p) = state.config.oauth.iter().find(|p| p.id() == id) {
        return p.name().to_string();
    }
    match id {
        "lichess" => "Lichess",
        "google" => "Google",
        "fake" => "Fake provider",
        other => other,
    }
    .to_string()
}

/// The ids of the providers this server is configured with. An identity
/// with any other provider can't be signed in with, so it is no way in.
fn configured(state: &AppState) -> Vec<String> {
    state
        .config
        .oauth
        .iter()
        .map(|p| p.id().to_string())
        .collect()
}

async fn account(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
) -> Result<Json<Account>, AuthError> {
    let auth = registered(&state, &user)?;
    Ok(Json(load_account(&state, auth, &user).await?))
}

/// The signed-in account as `GET /api/me/account` shows it.
pub(crate) async fn load_account(
    state: &AppState,
    auth: &Auth,
    user: &User,
) -> Result<Account, AuthError> {
    let row = sqlx::query!(
        r#"SELECT password_hash IS NOT NULL AS "has_password!", email,
                  (SELECT t.email FROM email_tokens t
                   WHERE t.user_id = u.id AND t.purpose = 'verify' AND t.expires_at > now()
                   ORDER BY t.created_at DESC LIMIT 1) AS pending_email
           FROM users u WHERE id = $1"#,
        user.id
    )
    .fetch_one(auth.db().pool())
    .await?;
    let identities = sqlx::query!(
        "SELECT provider, subject, label FROM identities WHERE user_id = $1
         ORDER BY created_at, provider, subject",
        user.id
    )
    .fetch_all(auth.db().pool())
    .await?
    .into_iter()
    .map(|row| LinkedIdentity {
        provider_name: provider_name(state, &row.provider),
        provider: row.provider,
        subject: row.subject,
        label: row.label,
    })
    .collect();
    Ok(Account {
        username: user.username.clone().unwrap_or_default(),
        has_password: row.has_password,
        email: row.email,
        pending_email: row.pending_email,
        identities,
    })
}

async fn disconnect(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
    Path((provider, subject)): Path<(String, String)>,
) -> Result<StatusCode, AuthError> {
    let auth = registered(&state, &user)?;
    let mut tx = auth.db().pool().begin().await?;
    // Lock the user row: a concurrent disconnect of the *other* method waits
    // here and then sees this one gone, so the two can't both pass the check.
    let has_password = sqlx::query_scalar!(
        r#"SELECT password_hash IS NOT NULL AS "has!" FROM users WHERE id = $1 FOR UPDATE"#,
        user.id
    )
    .fetch_one(&mut *tx)
    .await?;
    // Other identities count as a way in only while their provider is
    // switched on; the one being removed needs no provider to go.
    let counts = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE provider = $2 AND subject = $3) AS "this!",
                  count(*) FILTER (WHERE NOT (provider = $2 AND subject = $3)
                                   AND provider = ANY($4::text[])) AS "others!"
           FROM identities WHERE user_id = $1"#,
        user.id,
        provider,
        subject,
        &configured(&state)
    )
    .fetch_one(&mut *tx)
    .await?;
    if counts.this == 0 {
        return Err(AuthError::NoSuchIdentity);
    }
    if !has_password && counts.others == 0 {
        return Err(AuthError::LastWayIn);
    }
    sqlx::query!(
        "DELETE FROM identities WHERE user_id = $1 AND provider = $2 AND subject = $3",
        user.id,
        provider,
        subject
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    tracing::info!(
        "{} disconnected {provider}",
        user.username.as_deref().unwrap_or("?")
    );
    Ok(StatusCode::NO_CONTENT)
}

async fn set_password(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
    SessionId(session): SessionId,
    ip: ClientIp,
    Json(change): Json<PasswordChange>,
) -> Result<StatusCode, AuthError> {
    let auth = registered(&state, &user)?;
    // Guessing the current password here is guessing it at the login form,
    // so it spends the login form's attempts: holding a session buys no
    // extra guesses.
    auth.limit(&format!("login:{}", ip.key()), LOGIN_PER_ADDR)?;
    let username = user.username.as_deref().unwrap_or_default();
    auth.limit(
        &format!("login:{}", username.to_lowercase()),
        LOGIN_PER_USER,
    )?;
    if !valid_password(&change.password) {
        return Err(AuthError::WeakPassword);
    }
    let pool = auth.db().pool();
    let current_hash =
        sqlx::query_scalar!("SELECT password_hash FROM users WHERE id = $1", user.id)
            .fetch_one(pool)
            .await?;
    let first = current_hash.is_none();
    match current_hash {
        Some(hash) => {
            let given = change.current.unwrap_or_default();
            if !verify_password(given, hash).await {
                return Err(AuthError::WrongPassword);
            }
        }
        // With no password to confirm, the session is the only proof, and a
        // stolen one would do. So it has to come from a sign-in just now.
        None => {
            require_fresh_sign_in(&state, auth, &user, session.as_deref(), "set a password").await?
        }
    }
    let hash = hash_password(change.password).await;
    let mut tx = pool.begin().await?;
    // A first password is set only while there still is none, so a second
    // tab racing this one can't skip the current-password check.
    let updated = sqlx::query!(
        "UPDATE users SET password_hash = $2
         WHERE id = $1 AND (password_hash IS NULL) = $3",
        user.id,
        hash,
        first
    )
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AuthError::WrongPassword);
    }
    // Anyone else signed in as this account (the reason to change a
    // password, often) is signed out; this browser stays in. Links they had
    // mailed themselves go too: a verification of their own address,
    // followed later, would take the account's password resets.
    sqlx::query!(
        "DELETE FROM sessions WHERE user_id = $1 AND id IS DISTINCT FROM $2",
        user.id,
        session
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("DELETE FROM email_tokens WHERE user_id = $1", user.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Refuse with [`AuthError::SignInAgain`] unless `session` comes from a
/// sign-in in the last [`FRESH_SIGN_IN_MINUTES`]. This guards whatever would
/// give the holder of a stolen (older) session cookie a way in of their own:
/// a first password, another provider identity. `to` finishes the refusal's
/// "sign in again with X to …".
pub(crate) async fn require_fresh_sign_in(
    state: &AppState,
    auth: &Auth,
    user: &User,
    session: Option<&str>,
    to: &'static str,
) -> Result<(), AuthError> {
    if auth
        .signed_in_within(session, FRESH_SIGN_IN_MINUTES)
        .await?
    {
        return Ok(());
    }
    Err(AuthError::SignInAgain {
        provider: sign_in_again_with(state, auth, user).await?,
        to,
    })
}

/// The provider to send the account back through for a fresh sign-in: the
/// oldest of its identities whose provider is switched on.
pub(crate) async fn sign_in_again_with(
    state: &AppState,
    auth: &Auth,
    user: &User,
) -> Result<Option<ProviderInfo>, AuthError> {
    let provider = sqlx::query_scalar!(
        "SELECT provider FROM identities WHERE user_id = $1 AND provider = ANY($2::text[])
         ORDER BY created_at, provider, subject LIMIT 1",
        user.id,
        &configured(state)
    )
    .fetch_optional(auth.db().pool())
    .await?;
    Ok(provider.map(|id| ProviderInfo {
        name: provider_name(state, &id),
        id,
    }))
}
