//! Accounts phase D: rate limits, the proxy address header, session and
//! guest cleanup, and the same-origin check on the game socket.

mod common;

use common::*;
use server::{AppState, Config};
use tokio_tungstenite::tungstenite::{Error, http::StatusCode};

fn unique(prefix: &str) -> String {
    format!(
        "{prefix}{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    )
}

#[tokio::test]
async fn login_is_rate_limited_per_username() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    let name = unique("limited");
    let creds = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    assert_eq!(
        http(&base, "POST", "/api/auth/signup", None, &creds)
            .await
            .status,
        201
    );

    // Ten attempts per minute per username, then 429 with Retry-After,
    // even for the right password (the attacker doesn't get to find out).
    let wrong = format!(
        r#"{{"username":"{}","password":"wrong horse"}}"#,
        name.to_uppercase()
    );
    for _ in 0..9 {
        assert_eq!(
            http(&base, "POST", "/api/auth/login", None, &wrong)
                .await
                .status,
            401
        );
    }
    let r = http(&base, "POST", "/api/auth/login", None, &creds).await;
    assert_eq!(r.status, 200, "the tenth attempt is still served");
    let r = http(&base, "POST", "/api/auth/login", None, &creds).await;
    assert_eq!(r.status, 429, "{}", r.body);
    let wait: u64 = r
        .header("retry-after")
        .expect("Retry-After")
        .parse()
        .unwrap();
    assert!((1..=60).contains(&wait), "{wait}");
    assert!(r.body.contains("too many attempts"), "{}", r.body);

    // Another username from the same address is unaffected.
    let other = unique("other");
    let creds = format!(r#"{{"username":"{other}","password":"correct horse"}}"#);
    assert_eq!(
        http(&base, "POST", "/api/auth/signup", None, &creds)
            .await
            .status,
        201
    );
    assert_eq!(
        http(&base, "POST", "/api/auth/login", None, &creds)
            .await
            .status,
        200
    );
}

#[tokio::test]
async fn guests_are_rate_limited_per_forwarded_address_behind_a_proxy() {
    let Some(db) = db().await else { return };
    let base = serve_with(
        db,
        Config {
            trust_proxy: true,
            ..Config::default()
        },
    )
    .await;
    // The rightmost entry is the one the proxy appended; the client can put
    // whatever it likes on the left.
    let from = |ip: &str| [("X-Forwarded-For", format!("1.2.3.4, {ip}"))];
    for _ in 0..30 {
        let h = from("10.0.0.1");
        let r = http_with(
            &base,
            "POST",
            "/api/auth/guest",
            None,
            "",
            &[(h[0].0, &h[0].1)],
        )
        .await;
        assert_eq!(r.status, 200, "{}", r.body);
    }
    let h = from("10.0.0.1");
    let r = http_with(
        &base,
        "POST",
        "/api/auth/guest",
        None,
        "",
        &[(h[0].0, &h[0].1)],
    )
    .await;
    assert_eq!(r.status, 429, "{}", r.body);
    let h = from("10.0.0.2");
    let r = http_with(
        &base,
        "POST",
        "/api/auth/guest",
        None,
        "",
        &[(h[0].0, &h[0].1)],
    )
    .await;
    assert_eq!(r.status, 200, "another address is unaffected: {}", r.body);
}

#[tokio::test]
async fn cleanup_removes_expired_sessions_and_idle_guests_only() {
    let Some(db) = db().await else { return };
    let base = serve(db.clone()).await;
    let auth = AppState::with_db(db.clone(), Config::default())
        .auth
        .expect("accounts on");

    // Two guests, both a day old with expired sessions; one has a game.
    let idle = guest(&base).await;
    let player = guest(&base).await;
    create(&base, &player, "{}").await;
    for session in [&idle, &player] {
        sqlx::query(
            "UPDATE users SET created_at = now() - interval '2 days'
             WHERE id = (SELECT user_id FROM sessions WHERE id = $1)",
        )
        .bind(session)
        .execute(db.pool())
        .await
        .unwrap();
        sqlx::query("UPDATE sessions SET expires_at = now() - interval '1 second' WHERE id = $1")
            .bind(session)
            .execute(db.pool())
            .await
            .unwrap();
    }
    // Expired sessions are already unusable; the sweep just removes the rows.
    assert_eq!(
        http(&base, "GET", "/api/me", Some(&idle), "").await.status,
        401
    );

    let (sessions, guests) = auth.purge_expired().await.unwrap();
    assert!(sessions >= 2, "{sessions}");
    assert!(guests >= 1, "{guests}");

    let users_left: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM users WHERE id IN (
             SELECT white_user_id FROM games WHERE white_user_id IS NOT NULL)
           AND created_at < now() - interval '1 day' AND is_guest",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert!(users_left >= 1, "guests with games are kept");
    let dangling: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM users u WHERE u.is_guest
           AND u.created_at < now() - interval '1 day'
           AND NOT EXISTS (SELECT 1 FROM sessions s WHERE s.user_id = u.id)
           AND NOT EXISTS (SELECT 1 FROM games g WHERE g.white_user_id = u.id OR g.black_user_id = u.id)",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(dangling, 0);
}

#[tokio::test]
async fn game_sockets_refuse_foreign_origins() {
    let Some(db) = db().await else { return };
    let base = serve_with(
        db,
        Config {
            allowed_origins: vec!["https://chess.example".to_string()],
            ..Config::default()
        },
    )
    .await;
    let session = guest(&base).await;
    let game = create(&base, &session, "{}").await;

    // No Origin (a non-browser client), the request's own host, and the
    // configured public origin all connect.
    for origin in [
        None,
        Some(format!("http://{base}")),
        Some("https://chess.example".to_string()),
    ] {
        try_connect(&base, &game.id, Some(&session), origin.as_deref())
            .await
            .unwrap_or_else(|e| panic!("origin {origin:?}: {e}"));
    }
    // Any other page is refused before the upgrade.
    let Err(err) = try_connect(
        &base,
        &game.id,
        Some(&session),
        Some("https://evil.example"),
    )
    .await
    else {
        panic!("a foreign origin was let through");
    };
    match err {
        Error::Http(response) => assert_eq!(response.status(), StatusCode::FORBIDDEN),
        other => panic!("{other:?}"),
    }
}
