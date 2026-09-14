//! Accounts over HTTP: guest, signup (new and guest upgrade), login,
//! logout, `me`. Needs `TEST_DATABASE_URL`; skipped otherwise.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt as _;
use server::{AppState, Config, auth::User, db::Db};
use tower::ServiceExt as _;

async fn app() -> Option<Router> {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL not set; skipping");
        return None;
    };
    let db = Db::connect(&url).await.expect("connect");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("404.html"), "shell").unwrap();
    // Leak the temp dir for the test's lifetime; the router only reads it.
    let path = dir.keep();
    Some(server::app_with(
        path,
        AppState::with_db(db, Config::default()),
    ))
}

struct Reply {
    status: StatusCode,
    cookie: Option<String>,
    body: serde_json::Value,
}

async fn call(
    app: &Router,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    json: Option<&str>,
) -> Reply {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, format!("session={c}"));
    }
    let body = match json {
        Some(j) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(j.to_string())
        }
        None => Body::empty(),
    };
    let response = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    Reply {
        status,
        cookie,
        body,
    }
}

fn session_of(cookie: &str) -> String {
    let value = cookie.strip_prefix("session=").unwrap();
    value.split(';').next().unwrap().to_string()
}

fn unique(prefix: &str) -> String {
    format!(
        "{prefix}{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    )
}

#[tokio::test]
async fn guest_then_upgrade_then_login_elsewhere() {
    let Some(app) = app().await else { return };

    // Nobody yet.
    assert_eq!(
        call(&app, "GET", "/api/me", None, None).await.status,
        StatusCode::UNAUTHORIZED
    );

    // A guest gets a cookie with the right flags.
    let r = call(&app, "POST", "/api/auth/guest", None, None).await;
    assert_eq!(r.status, StatusCode::OK);
    let cookie = r.cookie.unwrap();
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax") && cookie.contains("Path=/"),
        "{cookie}"
    );
    assert!(
        !cookie.contains("Secure"),
        "dev cookies are not Secure: {cookie}"
    );
    let guest: User = serde_json::from_value(r.body).unwrap();
    assert!(guest.is_guest && guest.username.is_none());
    let session = session_of(&cookie);

    // Same guest on the next request; asking for a guest again is idempotent.
    let r = call(&app, "GET", "/api/me", Some(&session), None).await;
    assert_eq!(serde_json::from_value::<User>(r.body).unwrap(), guest);
    let r = call(&app, "POST", "/api/auth/guest", Some(&session), None).await;
    assert_eq!(r.cookie, None);
    assert_eq!(serde_json::from_value::<User>(r.body).unwrap(), guest);

    // Upgrade the guest: same id, now a real account.
    let name = unique("dan");
    let creds = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let r = call(
        &app,
        "POST",
        "/api/auth/signup",
        Some(&session),
        Some(&creds),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let user: User = serde_json::from_value(r.body).unwrap();
    assert_eq!(user.id, guest.id);
    assert_eq!(user.username.as_deref(), Some(name.as_str()));
    assert!(!user.is_guest);
    // The same session keeps working.
    let r = call(&app, "GET", "/api/me", Some(&session), None).await;
    assert_eq!(serde_json::from_value::<User>(r.body).unwrap(), user);

    // Log in from another device, case-insensitively.
    let creds = format!(
        r#"{{"username":"{}","password":"correct horse"}}"#,
        name.to_uppercase()
    );
    let r = call(&app, "POST", "/api/auth/login", None, Some(&creds)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let other = session_of(&r.cookie.unwrap());
    assert_ne!(other, session);
    let r = call(&app, "GET", "/api/me", Some(&other), None).await;
    assert_eq!(serde_json::from_value::<User>(r.body).unwrap(), user);

    // Wrong password, unknown user: same answer.
    let bad = format!(r#"{{"username":"{name}","password":"wrong horse"}}"#);
    assert_eq!(
        call(&app, "POST", "/api/auth/login", None, Some(&bad))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let nobody = r#"{"username":"nobody_here_xyz","password":"whatever1"}"#;
    assert_eq!(
        call(&app, "POST", "/api/auth/login", None, Some(nobody))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    // Logout revokes only that session.
    let r = call(&app, "POST", "/api/auth/logout", Some(&other), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(r.cookie.unwrap().contains("Max-Age=0"));
    assert_eq!(
        call(&app, "GET", "/api/me", Some(&other), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, "GET", "/api/me", Some(&session), None)
            .await
            .status,
        StatusCode::OK
    );

    // The name is taken now, case-insensitively.
    let dup = format!(
        r#"{{"username":"{}","password":"another one"}}"#,
        name.to_uppercase()
    );
    assert_eq!(
        call(&app, "POST", "/api/auth/signup", None, Some(&dup))
            .await
            .status,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn signup_validates_and_creates_fresh_users() {
    let Some(app) = app().await else { return };

    let r = call(
        &app,
        "POST",
        "/api/auth/signup",
        None,
        Some(r#"{"username":"ab","password":"long enough"}"#),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert!(r.body["error"].as_str().unwrap().contains("usernames"));
    let r = call(
        &app,
        "POST",
        "/api/auth/signup",
        None,
        Some(r#"{"username":"validname","password":"short"}"#),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert!(r.body["error"].as_str().unwrap().contains("passwords"));

    let name = unique("dillon");
    let creds = format!(r#"{{"username":"{name}","password":"a fine password"}}"#);
    let r = call(&app, "POST", "/api/auth/signup", None, Some(&creds)).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let user: User = serde_json::from_value(r.body).unwrap();
    assert!(!user.is_guest);
    let session = session_of(&r.cookie.unwrap());
    let r = call(&app, "GET", "/api/me", Some(&session), None).await;
    assert_eq!(serde_json::from_value::<User>(r.body).unwrap(), user);

    // A garbage session is just "not signed in".
    assert_eq!(
        call(&app, "GET", "/api/me", Some("deadbeef"), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn without_a_database_accounts_are_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("404.html"), "shell").unwrap();
    let app = server::app_with(dir.path(), AppState::in_memory());
    let r = call(&app, "POST", "/api/auth/guest", None, None).await;
    assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        call(&app, "GET", "/api/me", None, None).await.status,
        StatusCode::UNAUTHORIZED
    );
}
