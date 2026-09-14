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
