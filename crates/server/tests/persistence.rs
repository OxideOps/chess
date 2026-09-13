//! Games survive a "restart": a fresh `Games` registry on the same database
//! serves the game where the old one left off. Needs `TEST_DATABASE_URL`
//! (a Postgres the tests may write to); skipped otherwise.

use std::time::{Duration, Instant};

use chess_core::{
    Color,
    protocol::{ClientMessage, GameOverReason, GameResult, ServerMessage},
};
use futures_util::{SinkExt as _, StreamExt as _};
use server::{db::Db, games::Games, room::TimeControl};
use tokio_tungstenite::tungstenite::Message;

async fn db() -> Option<Db> {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL not set; skipping");
        return None;
    };
    Some(
        Db::connect(&url)
            .await
            .expect("connect to the test database"),
    )
}

fn mv(uci: &str) -> ClientMessage {
    ClientMessage::Move {
        uci: uci.parse().unwrap(),
    }
}

#[tokio::test]
async fn a_game_is_written_through_and_loaded_back() {
    let Some(db) = db().await else { return };
    let tc = TimeControl {
        initial: Duration::from_secs(60),
        increment: Duration::from_secs(1),
    };
    let created = Games::with_db(db.clone()).create(tc).await.unwrap();

    // Play directly against the room, then save, the way the socket handler does.
    let games = Games::with_db(db.clone());
    let snapshot_before = {
        let stored = db.load(&created.id).await.unwrap().unwrap();
        assert_eq!(stored.tokens[0], created.white_token);
        assert_eq!(stored.snapshot.moves, Vec::<String>::new());
        assert_eq!(stored.snapshot.white_ms, 60_000);
        let mut room =
            server::room::Room::restore(&stored.snapshot, stored.age, Instant::now()).unwrap();
        let t = Instant::now();
        room.handle(Some(Color::White), mv("e2e4"), t);
        room.handle(Some(Color::Black), mv("e7e5"), t);
        room.handle(Some(Color::White), ClientMessage::OfferDraw, t);
        let snap = room.snapshot(t);
        db.save_snapshot(&created.id, &snap).await.unwrap();
        snap
    };
    drop(games);

    let stored = db.load(&created.id).await.unwrap().unwrap();
    assert_eq!(stored.snapshot.moves, ["e2e4", "e7e5"]);
    assert_eq!(stored.snapshot.draw_offer, Some(Color::White));
    assert_eq!(stored.snapshot.ended, None);
    // The clock was running when saved and is still counted as running.
    assert!(stored.snapshot.clock_running_for_ms.is_some());
    assert_eq!(stored.snapshot.white_ms, snapshot_before.white_ms);

    assert!(db.load("no-such-game").await.unwrap().is_none());
}

#[tokio::test]
async fn a_restarted_server_serves_the_same_game() {
    let Some(db) = db().await else { return };

    // First server: create and play two moves over the socket.
    let (base, games) = serve(Games::with_db(db.clone())).await;
    let created = games
        .create(TimeControl {
            initial: Duration::from_secs(60),
            increment: Duration::ZERO,
        })
        .await
        .unwrap();
    let mut white = connect(&base, &created.id, Some(&created.white_token)).await;
    let mut black = connect(&base, &created.id, Some(&created.black_token)).await;
    recv(&mut white).await;
    recv(&mut black).await;
    send(&mut white, &mv("e2e4")).await;
    recv(&mut white).await;
    recv(&mut black).await;
    send(&mut black, &mv("e7e5")).await;
    recv(&mut white).await;
    recv(&mut black).await;
    drop((white, black));

    // "Restart": a new registry with nothing in memory, same database.
    let (base2, _games2) = serve(Games::with_db(db.clone())).await;
    let mut white = connect(&base2, &created.id, Some(&created.white_token)).await;
    match recv(&mut white).await {
        ServerMessage::Sync {
            moves,
            your_color,
            clocks,
            ..
        } => {
            assert_eq!(
                moves.iter().map(ToString::to_string).collect::<Vec<_>>(),
                ["e2e4", "e7e5"]
            );
            assert_eq!(your_color, Some(Color::White));
            assert!(
                clocks.white_ms <= 60_000 && clocks.white_ms > 50_000,
                "{clocks:?}"
            );
        }
        other => panic!("expected Sync, got {other:?}"),
    }
    // And it keeps going: resign, and the end is persisted.
    send(&mut white, &ClientMessage::Resign).await;
    assert!(matches!(
        recv(&mut white).await,
        ServerMessage::GameOver { end } if end.result == GameResult::BlackWins && end.reason == GameOverReason::Resignation
    ));
    // Give the write-through a moment, then check the row.
    let mut stored = None;
    for _ in 0..50 {
        stored = db.load(&created.id).await.unwrap();
        if stored.as_ref().is_some_and(|s| s.snapshot.ended.is_some()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let stored = stored.unwrap();
    assert_eq!(
        stored.snapshot.ended.unwrap().reason,
        GameOverReason::Resignation
    );
    assert_eq!(stored.snapshot.clock_running_for_ms, None);

    // A third server serves the finished game to a spectator.
    let (base3, _games3) = serve(Games::with_db(db.clone())).await;
    let mut watcher = connect(&base3, &created.id, None).await;
    assert!(matches!(
        recv(&mut watcher).await,
        ServerMessage::Sync {
            ended: Some(_),
            your_color: None,
            ..
        }
    ));
}

// ----- helpers (same shape as game_socket.rs) -----

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn serve(games: Games) -> (String, Games) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("404.html"), "shell").unwrap();
    let app = server::app_with(dir.path(), games.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _dir = dir;
        axum::serve(listener, app).await.unwrap();
    });
    (format!("127.0.0.1:{}", addr.port()), games)
}

async fn connect(base: &str, id: &str, token: Option<&str>) -> Socket {
    let url = match token {
        Some(t) => format!("ws://{base}/api/games/{id}/ws?token={t}"),
        None => format!("ws://{base}/api/games/{id}/ws"),
    };
    tokio_tungstenite::connect_async(url).await.unwrap().0
}

async fn send(socket: &mut Socket, msg: &ClientMessage) {
    socket
        .send(Message::Text(serde_json::to_string(msg).unwrap().into()))
        .await
        .unwrap();
}

async fn recv(socket: &mut Socket) -> ServerMessage {
    let msg = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("timed out")
        .expect("closed")
        .unwrap();
    match msg {
        Message::Text(text) => serde_json::from_str(&text).unwrap(),
        other => panic!("unexpected frame {other:?}"),
    }
}
