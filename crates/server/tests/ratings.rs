//! Rated games over real HTTP and sockets: who may play them, and that a
//! finished one moves both ratings exactly once.

mod common;

use std::time::Duration;

use chess_core::protocol::{Category, ClientMessage, GameOverReason, GameResult, ServerMessage};
use common::*;
use server::{Config, players::PlayerProfile};

fn unique(prefix: &str) -> String {
    format!(
        "{prefix}{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    )
}

/// A new account's session id.
async fn signup(base: &str, name: &str) -> String {
    let body = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let r = http(base, "POST", "/api/auth/signup", None, &body).await;
    assert_eq!(r.status, 201, "{}", r.body);
    r.cookie.unwrap()
}

async fn profile(base: &str, name: &str) -> PlayerProfile {
    let r = http(base, "GET", &format!("/api/players/{name}"), None, "").await;
    assert_eq!(r.status, 200, "{}", r.body);
    serde_json::from_str(&r.body).unwrap()
}

/// The next message within `ms`, if any.
async fn within(socket: &mut Socket, ms: u64) -> Option<ServerMessage> {
    tokio::time::timeout(Duration::from_millis(ms), recv_game(socket))
        .await
        .ok()
}

/// Both players connected to `id`, past their Syncs.
async fn sit(base: &str, id: &str, white: &str, black: &str) -> (Socket, Socket) {
    let mut w = connect(base, id, Some(white)).await;
    let mut b = connect(base, id, Some(black)).await;
    recv_game(&mut w).await;
    recv_game(&mut b).await;
    (w, b)
}

async fn e4_e5(w: &mut Socket, b: &mut Socket) {
    send(w, &mv("e2e4")).await;
    recv_game(w).await;
    recv_game(b).await;
    send(b, &mv("e7e5")).await;
    recv_game(w).await;
    recv_game(b).await;
}

#[tokio::test]
async fn a_rated_game_moves_both_ratings_once() {
    let Some(db) = db().await else { return };
    let base = serve(db.clone()).await;
    let (alice, bob) = (unique("alice"), unique("bob"));
    let alice_session = signup(&base, &alice).await;
    let bob_session = signup(&base, &bob).await;

    // Rated needs an account, on both sides.
    let guest_session = guest(&base).await;
    let r = http(
        &base,
        "POST",
        "/api/games",
        Some(&guest_session),
        r#"{"rated":true}"#,
    )
    .await;
    assert_eq!(r.status, 403, "{}", r.body);
    let game = create(&base, &alice_session, r#"{"rated":true}"#).await;
    let r = join(&base, &guest_session, &game.id).await;
    assert_eq!(r.status, 403, "{}", r.body);
    assert!(r.body.contains("rated game"), "{}", r.body);

    // New accounts sit down at 1500, provisional.
    let r = join(&base, &bob_session, &game.id).await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        r.body
            .matches(r#""rating":{"value":1500,"provisional":true}"#)
            .count(),
        2,
        "{}",
        r.body
    );

    let mut w = connect(&base, &game.id, Some(&alice_session)).await;
    match recv_game(&mut w).await {
        ServerMessage::Sync {
            rated,
            category,
            rating_diffs,
            ..
        } => {
            assert!(rated);
            assert_eq!(category, Category::Blitz); // the default 5+0
            assert_eq!(rating_diffs, None);
        }
        other => panic!("{other:?}"),
    }
    let mut b = connect(&base, &game.id, Some(&bob_session)).await;
    recv_game(&mut b).await;
    e4_e5(&mut w, &mut b).await;

    send(&mut b, &ClientMessage::Resign).await;
    for s in [&mut w, &mut b] {
        assert!(matches!(
            recv_game(s).await,
            ServerMessage::GameOver { end } if end.result == GameResult::WhiteWins
        ));
        match recv_game(s).await {
            ServerMessage::RatingsChanged { diffs } => {
                assert!(diffs.white > 0 && diffs.black == -diffs.white, "{diffs:?}");
            }
            other => panic!("{other:?}"),
        }
    }

    let a = profile(&base, &alice).await;
    assert_eq!(a.ratings.len(), 1);
    assert_eq!(a.ratings[0].category, Category::Blitz);
    assert_eq!(a.ratings[0].games, 1);
    assert!(a.ratings[0].rating > 1500 && a.ratings[0].provisional);
    let before = (a, profile(&base, &bob).await);
    assert!(before.1.ratings[0].rating < 1500);

    // Applying again (another end signal, a restart) changes nothing.
    assert_eq!(db.apply_ratings(&game.id).await.unwrap(), None);
    assert_eq!(
        (profile(&base, &alice).await, profile(&base, &bob).await),
        before
    );

    // The list and a fresh server both know the result.
    let r = http(&base, "GET", "/api/me/games", Some(&alice_session), "").await;
    assert!(
        r.body.contains(r#""rated":true"#) && r.body.contains(r#""rating_diffs":{"white":"#),
        "{}",
        r.body
    );
    let fresh = serve(db).await;
    let mut again = connect(&fresh, &game.id, Some(&alice_session)).await;
    assert!(matches!(
        recv_game(&mut again).await,
        ServerMessage::Sync {
            rating_diffs: Some(_),
            rated: true,
            ..
        }
    ));

    // Profiles are case-insensitive; unknown names are 404.
    assert_eq!(profile(&base, &alice.to_uppercase()).await.username, alice);
    let r = http(&base, "GET", "/api/players/nobody_here_xyz", None, "").await;
    assert_eq!(r.status, 404);
}

#[tokio::test]
async fn casual_and_aborted_games_leave_ratings_alone() {
    let Some(db) = db().await else { return };
    let base = serve_with(
        db,
        Config {
            abandon_after: Some(Duration::from_millis(200)),
            ..Config::default()
        },
    )
    .await;
    let (carol, dave) = (unique("carol"), unique("dave"));
    let carol_session = signup(&base, &carol).await;
    let dave_session = signup(&base, &dave).await;

    // Casual: finished, but no rating message and no rating.
    let game = create(&base, &carol_session, "{}").await;
    join(&base, &dave_session, &game.id).await;
    let (mut w, mut b) = sit(&base, &game.id, &carol_session, &dave_session).await;
    e4_e5(&mut w, &mut b).await;
    send(&mut w, &ClientMessage::Resign).await;
    assert!(matches!(
        recv_game(&mut b).await,
        ServerMessage::GameOver { .. }
    ));
    assert_eq!(within(&mut b, 300).await, None);
    drop((w, b));

    // Rated but aborted: Dave leaves before both have moved.
    let game = create(&base, &carol_session, r#"{"rated":true}"#).await;
    join(&base, &dave_session, &game.id).await;
    let (mut w, b) = sit(&base, &game.id, &carol_session, &dave_session).await;
    send(&mut w, &mv("e2e4")).await;
    drop(b);
    loop {
        match recv_game(&mut w).await {
            ServerMessage::GameOver { end } => {
                assert_eq!(end.result, GameResult::Aborted);
                assert_eq!(end.reason, GameOverReason::Abandoned);
                break;
            }
            ServerMessage::MovePlayed { .. } => continue,
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(within(&mut w, 300).await, None);

    assert!(profile(&base, &carol).await.ratings.is_empty());
    assert!(profile(&base, &dave).await.ratings.is_empty());
}
