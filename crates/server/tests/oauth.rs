//! Accounts phase E: signing in through an OAuth provider, driven end to end
//! against the built-in fake provider over real HTTP.

mod common;

use chess_core::protocol::ServerMessage;
use common::*;
use server::{Config, auth::User, oauth::Provider};

fn unique(prefix: &str) -> String {
    format!(
        "{prefix}{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    )
}

async fn oauth_server() -> Option<String> {
    let db = db().await?;
    Some(
        serve_with(
            db,
            Config {
                oauth: vec![Provider::fake()],
                ..Config::default()
            },
        )
        .await,
    )
}

/// The `oauth` flow cookie a reply set, as `oauth=<value>`.
fn flow_cookie(r: &Reply) -> String {
    let line = r
        .head
        .lines()
        .find(|l| l.starts_with("set-cookie: oauth="))
        .expect("flow cookie");
    line["set-cookie: ".len()..]
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

/// Walk the flow: start → provider (signing in as `name`) → callback, sending
/// `session` along if given. Returns the callback reply.
async fn sign_in(base: &str, name: &str, next: &str, session: Option<&str>, deny: bool) -> Reply {
    let start = http(
        base,
        "GET",
        &format!("/api/auth/fake/start?next={next}"),
        session,
        "",
    )
    .await;
    assert_eq!(start.status, 303, "{}", start.body);
    let flow = flow_cookie(&start);
    let authorize = start.header("location").expect("redirect to the provider");
    assert!(
        authorize.starts_with("/api/auth/fake-provider/authorize?"),
        "{authorize}"
    );
    assert!(
        authorize.contains("code_challenge_method=S256") && authorize.contains("&state="),
        "{authorize}"
    );
    let extra = if deny { "&deny=1" } else { "" };
    let provider = http(
        base,
        "GET",
        &format!("{authorize}&as={name}{extra}"),
        None,
        "",
    )
    .await;
    assert_eq!(provider.status, 303, "{}", provider.body);
    let callback = provider.header("location").expect("redirect back");
    let prefix = format!("http://{base}");
    let path = callback.strip_prefix(&prefix).expect(callback);
    let cookies = match session {
        Some(s) => format!("session={s}; {flow}"),
        None => flow,
    };
    http_with(base, "GET", path, None, "", &[("Cookie", &cookies)]).await
}

async fn me(base: &str, session: &str) -> User {
    let r = http(base, "GET", "/api/me", Some(session), "").await;
    assert_eq!(r.status, 200, "{}", r.body);
    serde_json::from_str(&r.body).unwrap()
}

#[tokio::test]
async fn provider_sign_in_creates_then_finds_the_user() {
    let Some(base) = oauth_server().await else {
        return;
    };
    let r = http(base.as_str(), "GET", "/api/auth/providers", None, "").await;
    assert_eq!(r.body, r#"[{"id":"fake","name":"Fake provider"}]"#);
    assert_eq!(
        http(&base, "GET", "/api/auth/nope/start", None, "")
            .await
            .status,
        404
    );

    let name = unique("Alice");
    let r = sign_in(&base, &name, "/games", None, false).await;
    assert_eq!(r.status, 303, "{}", r.body);
    assert_eq!(r.header("location"), Some("/games"));
    let session = r.cookie.expect("signed in");
    assert!(
        r.head.contains("set-cookie: oauth=;") || r.head.contains("Max-Age=0"),
        "flow cookie is removed: {}",
        r.head
    );
    let alice = me(&base, &session).await;
    assert_eq!(alice.username.as_deref(), Some(name.as_str()));
    assert!(!alice.is_guest);

    // The same provider account is the same user, case-insensitively.
    let r = sign_in(&base, &name.to_uppercase(), "/", None, false).await;
    let again = me(&base, &r.cookie.unwrap()).await;
    assert_eq!(again, alice);

    // No password was set: logging in with one fails.
    let creds = format!(r#"{{"username":"{name}","password":"anything at all"}}"#);
    assert_eq!(
        http(&base, "POST", "/api/auth/login", None, &creds)
            .await
            .status,
        401
    );
}

#[tokio::test]
async fn a_guest_is_upgraded_and_a_taken_name_gets_a_suffix() {
    let Some(base) = oauth_server().await else {
        return;
    };
    let guest_session = guest(&base).await;
    let game = create(&base, &guest_session, "{}").await;
    let guest_user = me(&base, &guest_session).await;

    let name = unique("Bob");
    let r = sign_in(&base, &name, "/", Some(&guest_session), false).await;
    assert_eq!(r.status, 303, "{}", r.body);
    assert!(
        r.cookie.as_deref().is_none_or(|c| c == guest_session),
        "the guest keeps their own session: {:?}",
        r.cookie
    );
    let bob = me(&base, &guest_session).await;
    assert_eq!(bob.id, guest_user.id);
    assert_eq!(bob.username.as_deref(), Some(name.as_str()));
    assert!(!bob.is_guest);
    let r = http(&base, "GET", "/api/me/games", Some(&guest_session), "").await;
    assert!(r.body.contains(&game.id), "the game came along: {}", r.body);
    // The seat in the running game now carries the name too.
    let mut socket = connect(&base, &game.id, Some(&guest_session)).await;
    match recv(&mut socket).await {
        ServerMessage::Sync { players, .. } => {
            assert_eq!(
                players.white.unwrap().username.as_deref(),
                Some(name.as_str())
            );
        }
        other => panic!("{other:?}"),
    }

    // Someone registers `carol…` with a password; a provider account of the
    // same name lands on `carol…_NNNN`.
    let taken = unique("carol");
    let creds = format!(r#"{{"username":"{taken}","password":"correct horse"}}"#);
    assert_eq!(
        http(&base, "POST", "/api/auth/signup", None, &creds)
            .await
            .status,
        201
    );
    let r = sign_in(&base, &taken, "/", None, false).await;
    let carol2 = me(&base, &r.cookie.unwrap()).await;
    let username = carol2.username.unwrap();
    assert!(
        username.starts_with(&taken) && username.len() == taken.len() + 5,
        "{username}"
    );
}

#[tokio::test]
async fn a_signed_in_account_links_the_identity() {
    let Some(base) = oauth_server().await else {
        return;
    };
    let name = unique("dave");
    let creds = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let r = http(&base, "POST", "/api/auth/signup", None, &creds).await;
    let session = r.cookie.unwrap();
    let dave = me(&base, &session).await;

    // Signed in as dave, "connect" a provider account with another name.
    let provider_name = unique("DaveOnLichess");
    let r = sign_in(&base, &provider_name, "/", Some(&session), false).await;
    assert_eq!(r.status, 303);
    assert_eq!(me(&base, &session).await, dave, "name and id unchanged");
    // From then on that provider account is dave.
    let r = sign_in(&base, &provider_name, "/", None, false).await;
    assert_eq!(me(&base, &r.cookie.unwrap()).await, dave);
}

#[tokio::test]
async fn failures_go_back_to_the_login_page() {
    let Some(base) = oauth_server().await else {
        return;
    };

    // Cancelled at the provider.
    let r = sign_in(&base, "nobody", "/games", None, true).await;
    assert_eq!(r.status, 303);
    let location = r.header("location").unwrap();
    assert!(
        location.starts_with("/login?error=") && location.contains("cancelled"),
        "{location}"
    );
    assert!(location.ends_with("&next=%2Fgames"), "{location}");
    assert_eq!(r.cookie, None);

    // The callback without the flow cookie, or with a state that doesn't match.
    let r = http(
        &base,
        "GET",
        "/api/auth/fake/callback?code=x&state=y",
        None,
        "",
    )
    .await;
    assert!(
        r.header("location").unwrap().contains("took+too+long"),
        "{}",
        r.head
    );
    let start = http(&base, "GET", "/api/auth/fake/start", None, "").await;
    let flow = flow_cookie(&start);
    let r = http_with(
        &base,
        "GET",
        "/api/auth/fake/callback?code=x&state=wrong",
        None,
        "",
        &[("Cookie", &flow)],
    )
    .await;
    assert!(
        r.header("location").unwrap().contains("did+not+match"),
        "{}",
        r.head
    );
    // A code the provider never issued (or one already used).
    let authorize = start.header("location").unwrap();
    let state = authorize
        .split('&')
        .find_map(|p| p.strip_prefix("state="))
        .unwrap();
    let r = http_with(
        &base,
        "GET",
        &format!("/api/auth/fake/callback?code=bogus&state={state}"),
        None,
        "",
        &[("Cookie", &flow)],
    )
    .await;
    assert!(
        r.header("location")
            .unwrap()
            .contains("unknown+or+used+code"),
        "{}",
        r.head
    );

    // Without the provider configured, nothing is offered.
    let Some(db) = db().await else { return };
    let plain = serve(db).await;
    assert_eq!(
        http(&plain, "GET", "/api/auth/providers", None, "")
            .await
            .body,
        "[]"
    );
    assert_eq!(
        http(&plain, "GET", "/api/auth/fake/start", None, "")
            .await
            .status,
        404
    );
}

async fn account(base: &str, session: &str) -> server::account::Account {
    let r = http(base, "GET", "/api/me/account", Some(session), "").await;
    assert_eq!(r.status, 200, "{}", r.body);
    serde_json::from_str(&r.body).unwrap()
}

fn labels(account: &server::account::Account) -> Vec<(String, Option<String>)> {
    account
        .identities
        .iter()
        .map(|i| (i.provider_name.clone(), i.label.clone()))
        .collect()
}

async fn put_password(base: &str, session: &str, body: &str) -> Reply {
    http(base, "PUT", "/api/me/password", Some(session), body).await
}

#[tokio::test]
async fn the_account_page_connects_and_disconnects_but_keeps_a_way_in() {
    let Some(base) = oauth_server().await else {
        return;
    };
    // Signed up through the provider: no password, one identity.
    let first = unique("Fran");
    let r = sign_in(&base, &first, "/", None, false).await;
    let session = r.cookie.unwrap();
    let fran = me(&base, &session).await;
    let username = fran.username.clone().unwrap();
    let profile_path = format!("/api/players/{username}");
    let profile = http(&base, "GET", &profile_path, None, "").await.body;
    let a = account(&base, &session).await;
    assert_eq!(a.username, username);
    assert!(!a.has_password);
    assert_eq!(
        labels(&a),
        vec![("Fake provider".to_string(), Some(first.clone()))]
    );
    let first_subject = a.identities[0].subject.clone();
    let path = |subject: &str| format!("/api/me/identities/fake/{subject}");

    // The only way in cannot be removed, and the refusal says why.
    let r = http(&base, "DELETE", &path(&first_subject), Some(&session), "").await;
    assert_eq!(r.status, 409, "{}", r.body);
    assert!(r.body.contains("only way into your account"), "{}", r.body);

    // Connect a second provider account while signed in: same user.
    let second = unique("FranElsewhere");
    let r = sign_in(&base, &second, "/account", Some(&session), false).await;
    assert_eq!(r.status, 303);
    assert_eq!(r.header("location"), Some("/account"));
    assert!(
        r.cookie.as_deref().is_none_or(|c| c == session),
        "keeps the session: {:?}",
        r.cookie
    );
    assert_eq!(me(&base, &session).await, fran);
    assert_eq!(
        labels(&account(&base, &session).await),
        vec![
            ("Fake provider".to_string(), Some(first.clone())),
            ("Fake provider".to_string(), Some(second.clone()))
        ]
    );
    // Connecting it again is signing in again: same account, a new session
    // in place of the old one.
    let r = sign_in(&base, &second, "/account", Some(&session), false).await;
    assert_eq!(r.header("location"), Some("/account"));
    let old_session = session;
    let session = r.cookie.expect("a fresh session");
    assert_ne!(session, old_session);
    assert_eq!(
        http(&base, "GET", "/api/me", Some(&old_session), "")
            .await
            .status,
        401
    );
    assert_eq!(me(&base, &session).await, fran);
    assert_eq!(account(&base, &session).await.identities.len(), 2);

    // Signing in with the new one, signed out, lands on the same player,
    // with the same ratings; a changed spelling refreshes the label.
    let r = sign_in(&base, &second.to_uppercase(), "/", None, false).await;
    let other_session = r.cookie.unwrap();
    assert_eq!(me(&base, &other_session).await, fran);
    assert_eq!(
        http(&base, "GET", &profile_path, None, "").await.body,
        profile
    );
    assert_eq!(
        account(&base, &session).await.identities[1].label,
        Some(second.to_uppercase())
    );

    // Now the first can go, but then the second is the last way in.
    let r = http(&base, "DELETE", &path(&first_subject), Some(&session), "").await;
    assert_eq!(r.status, 204, "{}", r.body);
    let a = account(&base, &session).await;
    assert_eq!(a.identities.len(), 1);
    let last = a.identities[0].subject.clone();
    let r = http(&base, "DELETE", &path(&last), Some(&session), "").await;
    assert_eq!(r.status, 409, "{}", r.body);
    let r = http(&base, "DELETE", &path("nobody"), Some(&session), "").await;
    assert_eq!(r.status, 404, "{}", r.body);

    // A password is another way in: set it (there is none to confirm), and
    // then the last identity can go too.
    let r = put_password(&base, &session, r#"{"password":"short"}"#).await;
    assert_eq!(r.status, 400, "{}", r.body);
    let r = put_password(&base, &session, r#"{"password":"first password"}"#).await;
    assert_eq!(r.status, 204, "{}", r.body);
    assert!(account(&base, &session).await.has_password);
    assert_eq!(
        http(&base, "GET", "/api/me", Some(&other_session), "")
            .await
            .status,
        401,
        "setting a password signs out the other sessions"
    );
    let r = http(&base, "DELETE", &path(&last), Some(&session), "").await;
    assert_eq!(r.status, 204, "{}", r.body);
    assert!(account(&base, &session).await.identities.is_empty());
    let creds = format!(r#"{{"username":"{username}","password":"first password"}}"#);
    let r = http(&base, "POST", "/api/auth/login", None, &creds).await;
    assert_eq!(r.status, 200, "{}", r.body);
    let login_session = r.cookie.unwrap();

    // Changing it needs the current one.
    let r = put_password(&base, &session, r#"{"password":"second password"}"#).await;
    assert_eq!(r.status, 403, "{}", r.body);
    let body = r#"{"current":"wrong password","password":"second password"}"#;
    assert_eq!(put_password(&base, &session, body).await.status, 403);
    let body = r#"{"current":"first password","password":"second password"}"#;
    assert_eq!(put_password(&base, &session, body).await.status, 204);
    assert_eq!(me(&base, &session).await, fran);
    assert_eq!(
        http(&base, "GET", "/api/me", Some(&login_session), "")
            .await
            .status,
        401
    );
    let creds = format!(r#"{{"username":"{username}","password":"second password"}}"#);
    assert_eq!(
        http(&base, "POST", "/api/auth/login", None, &creds)
            .await
            .status,
        200
    );

    // Guests and nobody have no account page.
    let guest_session = guest(&base).await;
    assert_eq!(
        http(&base, "GET", "/api/me/account", Some(&guest_session), "")
            .await
            .status,
        403
    );
    assert_eq!(
        http(&base, "GET", "/api/me/account", None, "").await.status,
        401
    );
}

#[tokio::test]
async fn connecting_someone_elses_provider_account_is_refused() {
    let Some(base) = oauth_server().await else {
        return;
    };
    let theirs = unique("Gil");
    let r = sign_in(&base, &theirs, "/", None, false).await;
    let gil = me(&base, &r.cookie.unwrap()).await;

    let name = unique("hana");
    let creds = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let session = http(&base, "POST", "/api/auth/signup", None, &creds)
        .await
        .cookie
        .unwrap();
    let hana = me(&base, &session).await;

    // Hana, signed in, tries to connect Gil's provider account: refused, back
    // on the account page with the reason, and still Hana.
    let r = sign_in(&base, &theirs, "/account", Some(&session), false).await;
    assert_eq!(r.status, 303);
    let location = r.header("location").unwrap();
    assert!(
        location.starts_with("/account?error=Connecting+Fake+provider+failed")
            && location.contains("already+signs+in+another+player"),
        "{location}"
    );
    assert_eq!(r.cookie, None);
    assert_eq!(me(&base, &session).await, hana);
    assert!(account(&base, &session).await.identities.is_empty());
    // And the provider account still signs Gil in.
    let r = sign_in(&base, &theirs, "/", None, false).await;
    assert_eq!(me(&base, &r.cookie.unwrap()).await, gil);

    // Cancelling a connect also comes back to the account page.
    let r = sign_in(&base, "whoever", "/account", Some(&session), true).await;
    let location = r.header("location").unwrap();
    assert!(
        location.starts_with("/account?error=") && location.contains("cancelled"),
        "{location}"
    );
}

/// Make `session` look as if it was signed in `minutes` ago.
async fn age_session(session: &str, minutes: i32) {
    let db = db().await.expect("the test database");
    sqlx::query("UPDATE sessions SET created_at = now() - make_interval(mins => $2) WHERE id = $1")
        .bind(session)
        .bind(minutes)
        .execute(db.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_first_password_needs_a_fresh_sign_in() {
    let Some(base) = oauth_server().await else {
        return;
    };
    let name = unique("Ines");
    let session = sign_in(&base, &name, "/", None, false)
        .await
        .cookie
        .unwrap();
    let ines = me(&base, &session).await;
    let username = ines.username.clone().unwrap();
    // Another device, signed in the same way.
    let elsewhere = sign_in(&base, &name, "/", None, false)
        .await
        .cookie
        .unwrap();

    // A session from a sign-in eleven minutes ago (or a stolen cookie) can't
    // plant a first password; the refusal names the provider to go back
    // through.
    age_session(&session, 11).await;
    let r = put_password(&base, &session, r#"{"password":"planted password"}"#).await;
    assert_eq!(r.status, 403, "{}", r.body);
    let body: serde_json::Value = serde_json::from_str(&r.body).unwrap();
    assert_eq!(
        body["error"],
        "sign in again with Fake provider to set a password"
    );
    assert_eq!(
        body["sign_in_again"],
        serde_json::json!({"id": "fake", "name": "Fake provider"})
    );
    assert!(!account(&base, &session).await.has_password);
    let creds = format!(r#"{{"username":"{username}","password":"planted password"}}"#);
    assert_eq!(
        http(&base, "POST", "/api/auth/login", None, &creds)
            .await
            .status,
        401
    );

    // Signing in again through the provider, from the account page, gives a
    // new session in place of the stale one, and that one may.
    let r = sign_in(&base, &name, "/account", Some(&session), false).await;
    assert_eq!(r.header("location"), Some("/account"));
    let fresh = r.cookie.expect("a new session");
    assert_ne!(fresh, session);
    assert_eq!(
        http(&base, "GET", "/api/me", Some(&session), "")
            .await
            .status,
        401,
        "the stale session is gone"
    );
    assert_eq!(me(&base, &fresh).await, ines);
    // Nine minutes on it is still fresh.
    age_session(&fresh, 9).await;
    let r = put_password(&base, &fresh, r#"{"password":"first password"}"#).await;
    assert_eq!(r.status, 204, "{}", r.body);
    assert!(account(&base, &fresh).await.has_password);
    assert_eq!(
        http(&base, "GET", "/api/me", Some(&elsewhere), "")
            .await
            .status,
        401,
        "a first password signs out the other sessions"
    );
    let creds = format!(r#"{{"username":"{username}","password":"first password"}}"#);
    assert_eq!(
        http(&base, "POST", "/api/auth/login", None, &creds)
            .await
            .status,
        200
    );

    // Changing an existing one needs no fresh session, only the current
    // password.
    age_session(&fresh, 60).await;
    let r = put_password(&base, &fresh, r#"{"password":"second password"}"#).await;
    assert_eq!(r.status, 403, "{}", r.body);
    assert!(r.body.contains("current password"), "{}", r.body);
    let body = r#"{"current":"first password","password":"second password"}"#;
    assert_eq!(put_password(&base, &fresh, body).await.status, 204);
}

#[tokio::test]
async fn a_provider_that_is_switched_off_is_no_way_in() {
    let Some(base) = oauth_server().await else {
        return;
    };
    let Some(db) = db().await else { return };
    // The same database, with no provider configured.
    let plain = serve(db).await;

    // Two identities, both with the fake provider, and no password.
    let first = unique("Jude");
    let session = sign_in(&base, &first, "/", None, false)
        .await
        .cookie
        .unwrap();
    let second = unique("JudeElsewhere");
    sign_in(&base, &second, "/account", Some(&session), false).await;
    let a = account(&base, &session).await;
    assert_eq!(a.identities.len(), 2);
    let path = format!("/api/me/identities/fake/{}", a.identities[0].subject);

    // Where that provider is switched off, the other identity can't sign
    // anyone in, so this one is the last way in.
    let r = http(&plain, "DELETE", &path, Some(&session), "").await;
    assert_eq!(r.status, 409, "{}", r.body);
    assert!(r.body.contains("only way into your account"), "{}", r.body);
    // And there is no provider to send a stale session back through.
    age_session(&session, 11).await;
    let r = put_password(&plain, &session, r#"{"password":"first password"}"#).await;
    assert_eq!(r.status, 403, "{}", r.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&r.body).unwrap(),
        serde_json::json!({"error": "sign in again to set a password"})
    );

    // Where it is on, the other one counts.
    let r = http(&base, "DELETE", &path, Some(&session), "").await;
    assert_eq!(r.status, 204, "{}", r.body);
    assert_eq!(account(&base, &session).await.identities.len(), 1);
}

#[tokio::test]
async fn the_password_form_spends_the_login_forms_attempts() {
    let Some(base) = oauth_server().await else {
        return;
    };
    let name = unique("Kit");
    let creds = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let session = http(&base, "POST", "/api/auth/signup", None, &creds)
        .await
        .cookie
        .unwrap();

    // Ten wrong guesses at the login form, under any spelling of the name...
    let wrong = format!(
        r#"{{"username":"{}","password":"wrong horse"}}"#,
        name.to_lowercase()
    );
    for _ in 0..10 {
        let r = http(&base, "POST", "/api/auth/login", None, &wrong).await;
        assert_eq!(r.status, 401, "{}", r.body);
    }
    // ...leave none for the password form, even with the right password.
    let body = r#"{"current":"correct horse","password":"new horse battery"}"#;
    let r = put_password(&base, &session, body).await;
    assert_eq!(r.status, 429, "{}", r.body);

    // And the other way round, on a fresh server: guesses through the
    // password form use up the login form's.
    let Some(base) = oauth_server().await else {
        return;
    };
    let name = unique("Lee");
    let creds = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let session = http(&base, "POST", "/api/auth/signup", None, &creds)
        .await
        .cookie
        .unwrap();
    let guess = r#"{"current":"wrong horse","password":"new horse battery"}"#;
    for _ in 0..10 {
        let r = put_password(&base, &session, guess).await;
        assert_eq!(r.status, 403, "{}", r.body);
    }
    let r = http(&base, "POST", "/api/auth/login", None, &creds).await;
    assert_eq!(r.status, 429, "{}", r.body);
}

/// Where a refused or failed flow sent the browser: the path, `?error=` and
/// `?sign_in_again=`, decoded.
fn refusal(r: &Reply) -> (String, String, Option<String>) {
    let location = r.header("location").expect("a redirect");
    let url = url::Url::parse("http://x").unwrap().join(location).unwrap();
    let param = |key: &str| {
        url.query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
    };
    (
        url.path().to_string(),
        param("error").unwrap_or_default(),
        param("sign_in_again"),
    )
}

#[tokio::test]
async fn connecting_a_provider_needs_a_fresh_sign_in() {
    let Some(base) = oauth_server().await else {
        return;
    };

    // An account with only a password, on a session from long ago (or a
    // stolen cookie): Connect is refused before the provider, back on the
    // account page, with no provider to name.
    let name = unique("Mo");
    let creds = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let session = http(&base, "POST", "/api/auth/signup", None, &creds)
        .await
        .cookie
        .unwrap();
    age_session(&session, 11).await;
    let r = http(
        &base,
        "GET",
        "/api/auth/fake/start?next=/account",
        Some(&session),
        "",
    )
    .await;
    assert_eq!(r.status, 303, "{}", r.body);
    assert!(!r.head.contains("set-cookie: oauth="), "{}", r.head);
    assert_eq!(
        refusal(&r),
        (
            "/account".into(),
            "Connecting Fake provider failed: sign in again to connect another sign-in method"
                .into(),
            Some(String::new())
        )
    );
    assert!(account(&base, &session).await.identities.is_empty());
    // Its password is how it signs in again; that session may connect.
    let fresh = http(&base, "POST", "/api/auth/login", None, &creds)
        .await
        .cookie
        .unwrap();
    let r = sign_in(&base, &unique("MoOnFake"), "/account", Some(&fresh), false).await;
    assert_eq!(r.header("location"), Some("/account"));
    assert_eq!(account(&base, &fresh).await.identities.len(), 1);

    // An account from a provider, on an old session: starting that provider
    // may be signing in again, so it goes ahead, but coming back with a new
    // identity is refused at the callback, naming the provider to use.
    let first = unique("Noor");
    let session = sign_in(&base, &first, "/", None, false)
        .await
        .cookie
        .unwrap();
    let noor = me(&base, &session).await;
    age_session(&session, 11).await;
    let thief = unique("Thief");
    let r = sign_in(&base, &thief, "/account", Some(&session), false).await;
    assert_eq!(r.cookie, None, "no new session");
    assert_eq!(
        refusal(&r),
        (
            "/account".into(),
            "Connecting Fake provider failed: sign in again with Fake provider to connect \
             another sign-in method"
                .into(),
            Some("fake".into())
        )
    );
    assert_eq!(account(&base, &session).await.identities.len(), 1);
    assert_eq!(me(&base, &session).await, noor, "still signed in");
    // Nor does the thief's provider account now sign in as anyone.
    let r = sign_in(&base, &thief, "/", None, false).await;
    assert_ne!(me(&base, &r.cookie.unwrap()).await.id, noor.id);

    // Signing in again with the identity it has works at any age...
    age_session(&session, 60 * 24).await;
    let r = sign_in(&base, &first, "/account", Some(&session), false).await;
    assert_eq!(r.header("location"), Some("/account"));
    let fresh = r.cookie.expect("a new session");
    assert_eq!(me(&base, &fresh).await, noor);
    // ...and the new session may connect another.
    let r = sign_in(
        &base,
        &unique("NoorElsewhere"),
        "/account",
        Some(&fresh),
        false,
    )
    .await;
    assert_eq!(r.header("location"), Some("/account"));
    assert_eq!(account(&base, &fresh).await.identities.len(), 2);

    // A guest's session of any age still upgrades through a provider.
    let guest_session = guest(&base).await;
    age_session(&guest_session, 60).await;
    let r = sign_in(&base, &unique("Olu"), "/", Some(&guest_session), false).await;
    assert_eq!(r.header("location"), Some("/"));
    assert!(!me(&base, &guest_session).await.is_guest);
}
