//! Games and the HTTP/WebSocket endpoints that expose them.
//!
//! - `POST /api/games` creates a game; the caller takes White.
//! - `POST /api/games/{id}/join` takes the open Black seat.
//! - `GET /api/games/{id}` is a JSON snapshot for debugging and tests.
//! - `GET /api/me/games` lists the caller's games, newest first.
//! - `GET /api/games/{id}/ws` is the game socket, authenticated by the
//!   session cookie: seat holders play, everyone else spectates. The server
//!   sends `Sync` first, then every `ServerMessage` the room produces.
//!
//! Rooms live in memory while the server runs. With a database
//! (`Games::with_db`) every change is written through and games not in
//! memory are loaded on first access, so they survive restarts.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chess_core::{
    Color,
    protocol::{ClientMessage, PlayerInfo, Players, ServerMessage},
};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::{
    AppState,
    auth::{CurrentUser, RequireUser, User},
    db::Db,
    room::{Outgoing, Room, TimeControl},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/games", post(create_game))
        .route("/api/games/{id}", get(snapshot))
        .route("/api/games/{id}/join", post(join_game))
        .route("/api/games/{id}/ws", get(connect))
        .route("/api/me/games", get(my_games))
}

/// Who holds a seat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat {
    pub user_id: String,
    pub username: Option<String>,
}

impl Seat {
    fn info(&self) -> PlayerInfo {
        PlayerInfo {
            username: self.username.clone(),
        }
    }
}

impl From<&User> for Seat {
    fn from(u: &User) -> Self {
        Seat {
            user_id: u.id.clone(),
            username: u.username.clone(),
        }
    }
}

/// Everything the endpoints share.
#[derive(Clone, Default)]
pub struct Games {
    inner: Arc<Mutex<HashMap<String, Arc<GameEntry>>>>,
    db: Option<Db>,
}

struct GameEntry {
    id: String,
    room: Mutex<Room>,
    seats: Mutex<[Option<Seat>; 2]>,
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CreatedGame {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub id: String,
    pub fen: String,
    pub movetext: String,
    pub clocks: chess_core::protocol::Clocks,
    pub ended: Option<chess_core::protocol::GameEnd>,
    pub players: Players,
}

/// One row of `GET /api/me/games`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GameListing {
    pub id: String,
    pub players: Players,
    /// Which seat the caller holds.
    #[serde(with = "chess_core::protocol::color")]
    #[cfg_attr(feature = "ts", ts(type = "\"white\" | \"black\""))]
    pub your_color: Color,
    pub ended: Option<chess_core::protocol::GameEnd>,
    pub moves: u32,
    /// ISO 8601.
    pub updated_at: String,
}

#[derive(Debug)]
pub enum JoinError {
    NotFound,
    /// The caller already sits on the other side.
    OwnGame,
    SeatTaken,
    Db(sqlx::Error),
}

impl Games {
    /// Games that are also written to Postgres.
    pub fn with_db(db: Db) -> Games {
        Games {
            inner: Arc::default(),
            db: Some(db),
        }
    }

    /// Create a game with `white` in the White seat.
    pub async fn create(
        &self,
        white: &User,
        time_control: TimeControl,
    ) -> Result<CreatedGame, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let seats = [Some(Seat::from(white)), None];
        if let Some(db) = &self.db {
            db.insert(&id, &seats, time_control).await?;
        }
        self.insert(&id, seats, Room::new(time_control));
        Ok(CreatedGame { id })
    }

    /// Take the open Black seat.
    pub async fn join(&self, id: &str, user: &User) -> Result<Players, JoinError> {
        let entry = self.get(id).await.ok_or(JoinError::NotFound)?;
        let players = {
            let mut seats = entry.seats.lock().unwrap();
            if seats[0].as_ref().is_some_and(|s| s.user_id == user.id) {
                return Err(JoinError::OwnGame);
            }
            match &seats[1] {
                // Already seated: nothing changes, nobody needs telling.
                Some(s) if s.user_id == user.id => return Ok(players_of(&seats)),
                Some(_) => return Err(JoinError::SeatTaken),
                None => seats[1] = Some(Seat::from(user)),
            }
            players_of(&seats)
        };
        if let Some(db) = &self.db {
            db.set_black(id, &user.id).await.map_err(JoinError::Db)?;
        }
        let _ = entry.tx.send(ServerMessage::PlayersChanged {
            players: players.clone(),
        });
        Ok(players)
    }

    fn insert(&self, id: &str, seats: [Option<Seat>; 2], room: Room) -> Arc<GameEntry> {
        let (tx, _) = broadcast::channel(64);
        let entry = Arc::new(GameEntry {
            id: id.to_string(),
            room: Mutex::new(room),
            seats: Mutex::new(seats),
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
        let entry = self.insert(id, stored.seats, room);
        // A restored game may already be past its deadline.
        entry.arm_flag_timer();
        Some(entry)
    }
}

fn players_of(seats: &[Option<Seat>; 2]) -> Players {
    Players {
        white: seats[0].as_ref().map(Seat::info),
        black: seats[1].as_ref().map(Seat::info),
    }
}

impl GameEntry {
    fn side_of(&self, user: Option<&User>) -> Option<Color> {
        let user = user?;
        let seats = self.seats.lock().unwrap();
        if seats[0].as_ref().is_some_and(|s| s.user_id == user.id) {
            Some(Color::White)
        } else if seats[1].as_ref().is_some_and(|s| s.user_id == user.id) {
            Some(Color::Black)
        } else {
            None
        }
    }

    fn players(&self) -> Players {
        players_of(&self.seats.lock().unwrap())
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

async fn create_game(
    State(games): State<Games>,
    RequireUser(user): RequireUser,
    body: Option<Json<CreateGame>>,
) -> Response {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let tc = TimeControl {
        initial: Duration::from_millis(body.initial_ms.unwrap_or(5 * 60 * 1000)),
        increment: Duration::from_millis(body.increment_ms.unwrap_or(0)),
    };
    match games.create(&user, tc).await {
        Ok(created) => (StatusCode::CREATED, Json(created)).into_response(),
        Err(e) => {
            tracing::error!("creating game: {e}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

async fn join_game(
    State(games): State<Games>,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Response {
    match games.join(&id, &user).await {
        Ok(players) => Json(players).into_response(),
        Err(JoinError::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(JoinError::OwnGame) => (
            StatusCode::BAD_REQUEST,
            "you are already playing White in this game",
        )
            .into_response(),
        Err(JoinError::SeatTaken) => (StatusCode::CONFLICT, "that seat is taken").into_response(),
        Err(JoinError::Db(e)) => {
            tracing::error!("joining game {id}: {e}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

async fn snapshot(State(games): State<Games>, Path(id): Path<String>) -> Response {
    let Some(entry) = games.get(&id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let players = entry.players();
    let room = entry.room.lock().unwrap();
    Json(GameSnapshot {
        id,
        fen: room.game().fen(),
        movetext: room.game().movetext(),
        clocks: room.clocks(Instant::now()),
        ended: room.ended(),
        players,
    })
    .into_response()
}

async fn my_games(State(games): State<Games>, RequireUser(user): RequireUser) -> Response {
    let Some(db) = &games.db else {
        return Json(Vec::<GameListing>::new()).into_response();
    };
    match db.games_of(&user.id).await {
        Ok(list) => Json(list).into_response(),
        Err(e) => {
            tracing::error!("listing games: {e}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

async fn connect(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<String>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    // A page on another site must not play with this browser's cookie.
    if !crate::origin::origin_allowed(&headers, &state.config.allowed_origins) {
        return (StatusCode::FORBIDDEN, "cross-origin socket refused").into_response();
    }
    let Some(entry) = state.games.get(&id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let side = entry.side_of(user.as_ref());
    ws.on_upgrade(move |socket| session(socket, entry, side))
}

async fn session(socket: WebSocket, entry: Arc<GameEntry>, side: Option<Color>) {
    let (mut sink, mut stream) = socket.split();
    let mut rx = entry.tx.subscribe();

    let sync = entry.sync(side);
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
                        let sync = entry.sync(side);
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

impl GameEntry {
    fn sync(&self, side: Option<Color>) -> ServerMessage {
        let players = self.players();
        let room = self.room.lock().unwrap();
        match room.sync(side, Instant::now()) {
            ServerMessage::Sync {
                start_fen,
                moves,
                clocks,
                your_color,
                ended,
                draw_offer,
                ..
            } => ServerMessage::Sync {
                start_fen,
                moves,
                clocks,
                your_color,
                ended,
                draw_offer,
                players,
            },
            other => other,
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
