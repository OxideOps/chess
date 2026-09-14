//! Shared helpers: a server on an ephemeral port, a tiny HTTP client with a
//! session cookie, and WebSocket helpers. Online games need accounts, so
//! everything here runs against `TEST_DATABASE_URL` and skips without it.
#![allow(dead_code)]

use std::time::Duration;

use chess_core::protocol::{ClientMessage, ServerMessage};
use futures_util::{SinkExt as _, StreamExt as _};
use server::{AppState, Config, db::Db, games::CreatedGame};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest as _},
};

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn db() -> Option<Db> {
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

/// Start a server with games and accounts on `db`; returns `host:port`.
pub async fn serve(db: Db) -> String {
    serve_with(db, Config::default()).await
}

pub async fn serve_with(db: Db, config: Config) -> String {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("404.html"), "shell").unwrap();
    let app = server::app_with(dir.path(), AppState::with_db(db, config))
        .into_make_service_with_connect_info::<std::net::SocketAddr>();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _dir = dir;
        axum::serve(listener, app).await.unwrap();
    });
    format!("127.0.0.1:{}", addr.port())
}

pub struct Reply {
    pub status: u16,
    pub body: String,
    pub cookie: Option<String>,
    /// The response headers, lowercased names, one per line.
    pub head: String,
}

impl Reply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.head
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{name}: ")))
    }
}

/// One HTTP/1.1 request; `cookie` is a session id.
pub async fn http(base: &str, method: &str, path: &str, cookie: Option<&str>, body: &str) -> Reply {
    http_with(base, method, path, cookie, body, &[]).await
}

/// [`http`] with extra request headers.
pub async fn http_with(
    base: &str,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: &str,
    extra: &[(&str, &str)],
) -> Reply {
    let mut stream = TcpStream::connect(base).await.unwrap();
    let cookie_line = cookie
        .map(|c| format!("Cookie: session={c}\r\n"))
        .unwrap_or_default();
    let extra_lines: String = extra.iter().map(|(k, v)| format!("{k}: {v}\r\n")).collect();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {base}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{cookie_line}{extra_lines}Connection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    let status: u16 = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    let cookie = head.lines().find_map(|l| {
        l.strip_prefix("set-cookie: session=")
            .map(|v| v.split(';').next().unwrap().to_string())
    });
    Reply {
        status,
        body: body.to_string(),
        cookie,
        head: head.to_string(),
    }
}

/// A fresh guest's session id.
pub async fn guest(base: &str) -> String {
    let r = http(base, "POST", "/api/auth/guest", None, "").await;
    assert_eq!(r.status, 200, "{}", r.body);
    r.cookie.expect("guest gets a session cookie")
}

pub async fn create(base: &str, session: &str, body: &str) -> CreatedGame {
    let r = http(base, "POST", "/api/games", Some(session), body).await;
    assert_eq!(r.status, 201, "{}", r.body);
    serde_json::from_str(&r.body).unwrap()
}

pub async fn join(base: &str, session: &str, id: &str) -> Reply {
    http(
        base,
        "POST",
        &format!("/api/games/{id}/join"),
        Some(session),
        "",
    )
    .await
}

pub async fn connect(base: &str, id: &str, session: Option<&str>) -> Socket {
    try_connect(base, id, session, None).await.unwrap()
}

/// Open the game socket, optionally as a browser would, with an `Origin`.
pub async fn try_connect(
    base: &str,
    id: &str,
    session: Option<&str>,
    origin: Option<&str>,
) -> Result<Socket, tokio_tungstenite::tungstenite::Error> {
    let mut request = format!("ws://{base}/api/games/{id}/ws")
        .into_client_request()
        .unwrap();
    if let Some(s) = session {
        request
            .headers_mut()
            .insert("Cookie", format!("session={s}").parse().unwrap());
    }
    if let Some(o) = origin {
        request.headers_mut().insert("Origin", o.parse().unwrap());
    }
    tokio_tungstenite::connect_async(request)
        .await
        .map(|(socket, _)| socket)
}

pub async fn send(socket: &mut Socket, msg: &ClientMessage) {
    socket
        .send(Message::Text(serde_json::to_string(msg).unwrap().into()))
        .await
        .unwrap();
}

pub async fn recv(socket: &mut Socket) -> ServerMessage {
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

pub fn mv(uci: &str) -> ClientMessage {
    ClientMessage::Move {
        uci: uci.parse().unwrap(),
    }
}
