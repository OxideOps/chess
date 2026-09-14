//! End-to-end: real TCP, real WebSockets, two players and a spectator.

use std::time::Duration;

use chess_core::protocol::{ClientMessage, GameOverReason, GameResult, ServerMessage};
use futures_util::{SinkExt as _, StreamExt as _};
use server::games::CreatedGame;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Start the server on an ephemeral port and return its base URL.
async fn serve() -> String {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("404.html"), "shell").unwrap();
    let app = server::app_with(dir.path(), server::AppState::in_memory());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        // Keep the temp dir alive as long as the server.
        let _dir = dir;
        axum::serve(listener, app).await.unwrap();
    });
    format!("127.0.0.1:{}", addr.port())
}

async fn create(base: &str, body: &str) -> CreatedGame {
    // A minimal HTTP client is enough for one POST.
    let mut stream = TcpStream::connect(base).await.unwrap();
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let req = format!(
        "POST /api/games HTTP/1.1\r\nHost: {base}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 201"), "{response}");
    let json = response.split("\r\n\r\n").nth(1).unwrap();
    serde_json::from_str(json).unwrap()
}

async fn connect(base: &str, id: &str, token: Option<&str>) -> Socket {
    let url = match token {
        Some(t) => format!("ws://{base}/api/games/{id}/ws?token={t}"),
        None => format!("ws://{base}/api/games/{id}/ws"),
    };
    let (socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    socket
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
        .expect("timed out waiting for a message")
        .expect("socket closed")
        .unwrap();
    match msg {
        Message::Text(text) => serde_json::from_str(&text).unwrap(),
        other => panic!("unexpected frame {other:?}"),
    }
}

fn mv(uci: &str) -> ClientMessage {
    ClientMessage::Move {
        uci: uci.parse().unwrap(),
    }
}

#[tokio::test]
async fn two_players_and_a_spectator() {
    let base = serve().await;
    let game = create(&base, "{}").await;

    let mut white = connect(&base, &game.id, Some(&game.white_token)).await;
    let mut black = connect(&base, &game.id, Some(&game.black_token)).await;
    let mut watcher = connect(&base, &game.id, None).await;

    // Everyone gets a Sync first, telling them who they are.
    assert!(matches!(
        recv(&mut white).await,
        ServerMessage::Sync { your_color: Some(chess_core::Color::White), ref moves, clocks, .. }
            if moves.is_empty() && clocks.white_ms == 300_000
    ));
    assert!(matches!(
        recv(&mut black).await,
        ServerMessage::Sync {
            your_color: Some(chess_core::Color::Black),
            ..
        }
    ));
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

    // A reconnecting player gets the game so far.
    drop(black);
    let mut black = connect(&base, &game.id, Some(&game.black_token)).await;
    assert!(matches!(
        recv(&mut black).await,
        ServerMessage::Sync { your_color: Some(chess_core::Color::Black), ref moves, .. }
            if moves.len() == 1
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

    // Unknown game: no upgrade.
    let err = tokio_tungstenite::connect_async(format!("ws://{base}/api/games/nope/ws")).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn the_server_flags_a_player_who_runs_out_of_time() {
    let base = serve().await;
    let game = create(&base, r#"{"initial_ms": 300, "increment_ms": 0}"#).await;
    let mut white = connect(&base, &game.id, Some(&game.white_token)).await;
    let mut black = connect(&base, &game.id, Some(&game.black_token)).await;
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
