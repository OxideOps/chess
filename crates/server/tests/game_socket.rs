//! End-to-end: real TCP, real WebSockets, cookie-authenticated seats.

mod common;

use std::time::Duration;

use chess_core::{
    Color,
    protocol::{ClientMessage, GameOverReason, GameResult, ServerMessage},
};
use common::*;
use futures_util::SinkExt as _;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn two_players_and_a_spectator() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    let white_session = guest(&base).await;
    let black_session = guest(&base).await;
    let game = create(&base, &white_session, "{}").await;

    // Before anyone joins, the creator is White and the other seat is open.
    let mut white = connect(&base, &game.id, Some(&white_session)).await;
    match recv(&mut white).await {
        ServerMessage::Sync {
            your_color,
            moves,
            clocks,
            players,
            ..
        } => {
            assert_eq!(your_color, Some(Color::White));
            assert!(moves.is_empty());
            assert_eq!(clocks.white_ms, 300_000);
            assert!(players.white.is_some() && players.black.is_none());
        }
        other => panic!("{other:?}"),
    }

    // The creator can't join their own game; the second guest takes Black,
    // and White hears about it.
    assert_eq!(join(&base, &white_session, &game.id).await.status, 400);
    assert_eq!(join(&base, &black_session, &game.id).await.status, 200);
    assert!(matches!(
        recv(&mut white).await,
        ServerMessage::PlayersChanged { players } if players.black.is_some()
    ));
    // Joining again is idempotent; a third person is refused.
    assert_eq!(join(&base, &black_session, &game.id).await.status, 200);
    let third = guest(&base).await;
    assert_eq!(join(&base, &third, &game.id).await.status, 409);
    assert_eq!(join(&base, &third, "no-such-game").await.status, 404);

    let mut black = connect(&base, &game.id, Some(&black_session)).await;
    assert!(matches!(
        recv(&mut black).await,
        ServerMessage::Sync {
            your_color: Some(Color::Black),
            ..
        }
    ));
    let mut watcher = connect(&base, &game.id, None).await;
    assert!(matches!(
        recv(&mut watcher).await,
        ServerMessage::Sync {
            your_color: None,
            ..
        }
    ));

    // Black can't move first; the spectator can't move at all.
    send(&mut black, &mv("e7e5")).await;
    assert!(
        matches!(recv(&mut black).await, ServerMessage::Rejected { message } if message == "it is not your turn")
    );
    send(&mut watcher, &mv("e2e4")).await;
    assert!(matches!(
        recv(&mut watcher).await,
        ServerMessage::Rejected { .. }
    ));

    // A legal move reaches all three.
    send(&mut white, &mv("e2e4")).await;
    for s in [&mut white, &mut black, &mut watcher] {
        assert!(matches!(
            recv(s).await,
            ServerMessage::MovePlayed { ply: 1, uci, .. } if uci.to_string() == "e2e4"
        ));
    }

    // Garbage is rejected without dropping the connection.
    white.send(Message::Text("not json".into())).await.unwrap();
    assert!(
        matches!(recv(&mut white).await, ServerMessage::Rejected { message } if message.starts_with("bad message"))
    );
    send(&mut white, &ClientMessage::Ping).await;
    assert_eq!(recv(&mut white).await, ServerMessage::Pong);

    // A reconnecting player gets the game so far and keeps their seat.
    drop(black);
    let mut black = connect(&base, &game.id, Some(&black_session)).await;
    assert!(matches!(
        recv(&mut black).await,
        ServerMessage::Sync { your_color: Some(Color::Black), ref moves, .. } if moves.len() == 1
    ));

    // Resigning ends it for everyone; then nothing more is accepted.
    send(&mut black, &ClientMessage::Resign).await;
    for s in [&mut white, &mut black, &mut watcher] {
        assert!(matches!(
            recv(s).await,
            ServerMessage::GameOver { end } if end.result == GameResult::WhiteWins && end.reason == GameOverReason::Resignation
        ));
    }
    send(&mut white, &mv("d2d4")).await;
    assert!(matches!(
        recv(&mut white).await,
        ServerMessage::Rejected { .. }
    ));

    // The game shows up in both players' lists, not the spectator's.
    let r = http(&base, "GET", "/api/me/games", Some(&white_session), "").await;
    assert_eq!(r.status, 200);
    assert!(
        r.body.contains(&game.id) && r.body.contains("resignation"),
        "{}",
        r.body
    );
    let r = http(&base, "GET", "/api/me/games", Some(&third), "").await;
    assert_eq!(r.body, "[]");

    // No session: no game; unknown game: no upgrade.
    assert_eq!(
        http(&base, "POST", "/api/games", None, "{}").await.status,
        401
    );
    let err = tokio_tungstenite::connect_async(format!("ws://{base}/api/games/nope/ws")).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn the_server_flags_a_player_who_runs_out_of_time() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    let ws = guest(&base).await;
    let bs = guest(&base).await;
    let game = create(&base, &ws, r#"{"initial_ms": 300, "increment_ms": 0}"#).await;
    join(&base, &bs, &game.id).await;
    let mut white = connect(&base, &game.id, Some(&ws)).await;
    let mut black = connect(&base, &game.id, Some(&bs)).await;
    recv(&mut white).await;
    recv(&mut black).await;

    // Clocks start after both have moved; then White lets theirs run out.
    send(&mut white, &mv("e2e4")).await;
    recv(&mut white).await;
    recv(&mut black).await;
    send(&mut black, &mv("e7e5")).await;
    recv(&mut white).await;
    recv(&mut black).await;

    let start = std::time::Instant::now();
    let msg = recv(&mut black).await;
    assert!(matches!(
        msg,
        ServerMessage::GameOver { end } if end.result == GameResult::BlackWins && end.reason == GameOverReason::Timeout
    ));
    assert!(
        start.elapsed() >= Duration::from_millis(250),
        "flagged too early"
    );
    assert!(matches!(
        recv(&mut white).await,
        ServerMessage::GameOver { .. }
    ));
}
