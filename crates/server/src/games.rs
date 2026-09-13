//! In-memory games and the HTTP/WebSocket endpoints that expose them.
//!
//! - `POST /api/games` creates a game and returns its id plus one secret
//!   token per side. Whoever connects with a token plays that side; anyone
//!   else spectates. (Accounts replace tokens later, roadmap 4.)
//! - `GET /api/games/{id}` is a JSON snapshot for debugging and tests.
//! - `GET /api/games/{id}/ws?token=…` is the game socket: the server sends
//!   `Sync` first, then every `ServerMessage` the room produces.
//!
//! With a database (`Games::with_db`) every change is written through and
//! games not in memory are loaded on first access, so they survive restarts.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chess_core::{
    Color,
    protocol::{ClientMessage, ServerMessage},
};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::{
    db::Db,
    room::{Outgoing, Room, TimeControl},
};

/// Everything the endpoints share.
#[derive(Clone, Default)]
pub struct Games {
    inner: Arc<Mutex<HashMap<String, Arc<GameEntry>>>>,
    db: Option<Db>,
}

struct GameEntry {
    id: String,
    room: Mutex<Room>,
    tokens: [String; 2],
    /// Fan-out of broadcast messages to every connection on this game.
    tx: broadcast::Sender<ServerMessage>,
    db: Option<Db>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CreateGame {
    /// Milliseconds per side; default 5 minutes.
    pub initial_ms: Option<u64>,
    /// Milliseconds added after each move; default 0.
    pub increment_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreatedGame {
    pub id: String,
    pub white_token: String,
    pub black_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub id: String,
    pub fen: String,
    pub movetext: String,
    pub clocks: chess_core::protocol::Clocks,
    pub ended: Option<chess_core::protocol::GameEnd>,
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    token: Option<String>,
}

impl Games {
    /// Games that are also written to Postgres.
    pub fn with_db(db: Db) -> Games {
        Games {
            inner: Arc::default(),
            db: Some(db),
        }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/api/games", post(create_game))
            .route("/api/games/{id}", get(snapshot))
            .route("/api/games/{id}/ws", get(connect))
            .with_state(self)
    }

    pub async fn create(&self, time_control: TimeControl) -> Result<CreatedGame, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let tokens = [
            uuid::Uuid::new_v4().to_string(),
            uuid::Uuid::new_v4().to_string(),
        ];
        if let Some(db) = &self.db {
            db.insert(&id, &tokens, time_control).await?;
        }
        self.insert(&id, tokens.clone(), Room::new(time_control));
        Ok(CreatedGame {
            id,
            white_token: tokens[0].clone(),
            black_token: tokens[1].clone(),
        })
    }

    fn insert(&self, id: &str, tokens: [String; 2], room: Room) -> Arc<GameEntry> {
        let (tx, _) = broadcast::channel(64);
        let entry = Arc::new(GameEntry {
            id: id.to_string(),
            room: Mutex::new(room),
            tokens,
            tx,
            db: self.db.clone(),
        });
        self.inner
            .lock()
            .unwrap()
            .entry(id.to_string())
            .or_insert(entry)
            .clone()
    }

    /// The game from memory, or from the database if it isn't loaded yet.
    async fn get(&self, id: &str) -> Option<Arc<GameEntry>> {
        if let Some(entry) = self.inner.lock().unwrap().get(id).cloned() {
            return Some(entry);
        }
        let db = self.db.as_ref()?;
        let stored = match db.load(id).await {
            Ok(stored) => stored?,
            Err(e) => {
                tracing::error!("loading game {id}: {e}");
                return None;
            }
        };
        let room = match Room::restore(&stored.snapshot, stored.age, Instant::now()) {
            Ok(room) => room,
            Err(e) => {
                tracing::error!("game {id} in the database doesn't replay: {e}");
                return None;
            }
        };
        let entry = self.insert(id, stored.tokens, room);
        // A restored game may already be past its deadline.
        entry.arm_flag_timer();
        Some(entry)
    }
}

impl GameEntry {
    fn side_for(&self, token: Option<&str>) -> Option<Color> {
        match token {
            Some(t) if t == self.tokens[0] => Some(Color::White),
            Some(t) if t == self.tokens[1] => Some(Color::Black),
            _ => None,
        }
    }

    /// Send what the room produced: broadcasts to everyone, replies to `reply`.
    /// Returns whether anything changed (broadcasts mean state changed).
    fn dispatch(&self, out: Vec<Outgoing>, reply: &mut Vec<ServerMessage>) -> bool {
        let mut changed = false;
        for o in out {
            match o {
                Outgoing::Reply(m) => reply.push(m),
                // No receivers just means nobody is connected.
                Outgoing::Broadcast(m) => {
                    changed = true;
                    let _ = self.tx.send(m);
                }
            }
        }
        changed
    }

    /// Write the room to the database, if there is one. Failures are logged:
    /// the game goes on in memory.
    async fn persist(&self) {
        let Some(db) = &self.db else { return };
        // Snapshot under the lock, write without it.
        let snapshot = {
            let room = self.room.lock().unwrap();
            room.snapshot(Instant::now())
        };
        if let Err(e) = db.save_snapshot(&self.id, &snapshot).await {
            tracing::error!("saving game {}: {e}", self.id);
        }
    }

    /// After a move, wake up when the mover's opponent would flag, and end
    /// the game then if the clock is still running.
    fn arm_flag_timer(self: &Arc<Self>) {
        let Some(deadline) = self.room.lock().unwrap().deadline() else {
            return;
        };
        let entry = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep_until((deadline + Duration::from_millis(1)).into()).await;
            let out = entry.room.lock().unwrap().check_timeout(Instant::now());
            if let Some(out) = out
                && entry.dispatch(vec![out], &mut Vec::new())
            {
                entry.persist().await;
            }
        });
    }
}

async fn create_game(State(games): State<Games>, body: Option<Json<CreateGame>>) -> Response {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let tc = TimeControl {
        initial: Duration::from_millis(body.initial_ms.unwrap_or(5 * 60 * 1000)),
        increment: Duration::from_millis(body.increment_ms.unwrap_or(0)),
    };
    match games.create(tc).await {
        Ok(created) => (StatusCode::CREATED, Json(created)).into_response(),
        Err(e) => {
            tracing::error!("creating game: {e}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

async fn snapshot(State(games): State<Games>, Path(id): Path<String>) -> Response {
    let Some(entry) = games.get(&id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let room = entry.room.lock().unwrap();
    Json(GameSnapshot {
        id,
        fen: room.game().fen(),
        movetext: room.game().movetext(),
        clocks: room.clocks(Instant::now()),
        ended: room.ended(),
    })
    .into_response()
}

async fn connect(
    State(games): State<Games>,
    Path(id): Path<String>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let Some(entry) = games.get(&id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let side = entry.side_for(query.token.as_deref());
    ws.on_upgrade(move |socket| session(socket, entry, side))
}

async fn session(socket: WebSocket, entry: Arc<GameEntry>, side: Option<Color>) {
    let (mut sink, mut stream) = socket.split();
    let mut rx = entry.tx.subscribe();

    let sync = entry.room.lock().unwrap().sync(side, Instant::now());
    if send(&mut sink, &sync).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            incoming = stream.next() => {
                let text = match incoming {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => continue, // binary/ping/pong frames
                };
                let mut replies = Vec::new();
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(msg) => {
                        let is_move = matches!(msg, ClientMessage::Move { .. });
                        let out = entry.room.lock().unwrap().handle(side, msg, Instant::now());
                        if entry.dispatch(out, &mut replies) {
                            entry.persist().await;
                        }
                        if is_move {
                            entry.arm_flag_timer();
                        }
                    }
                    Err(e) => replies.push(ServerMessage::Rejected {
                        message: format!("bad message: {e}"),
                    }),
                }
                for m in replies {
                    if send(&mut sink, &m).await.is_err() {
                        return;
                    }
                }
            }
            broadcast = rx.recv() => {
                match broadcast {
                    Ok(m) => {
                        if send(&mut sink, &m).await.is_err() {
                            return;
                        }
                    }
                    // Fell behind: resync rather than miss a move.
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let sync = entry.room.lock().unwrap().sync(side, Instant::now());
                        if send(&mut sink, &sync).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn send(
    sink: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &ServerMessage,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("protocol types serialize");
    sink.send(Message::Text(text.into())).await
}
