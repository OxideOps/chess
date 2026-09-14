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
    protocol::{
        Category, ClientMessage, PlayerInfo, PlayerRating, Players, RatingDiffs, ServerMessage,
    },
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
    /// Their rating in the game's category when they sat down; `None` for guests.
    pub rating: Option<PlayerRating>,
}

impl Seat {
    fn info(&self) -> PlayerInfo {
        PlayerInfo {
            username: self.username.clone(),
            rating: self.rating,
        }
    }
}

/// Everything the endpoints share.
/// How long a player who disconnects has to come back before the game is
/// aborted (nobody had moved yet) or lost by abandonment.
pub const DEFAULT_ABANDON_AFTER: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct Games {
    inner: Arc<Mutex<HashMap<String, Arc<GameEntry>>>>,
    db: Option<Db>,
    abandon_after: Duration,
}

impl Default for Games {
    fn default() -> Self {
        Games {
            inner: Arc::default(),
            db: None,
            abandon_after: DEFAULT_ABANDON_AFTER,
        }
    }
}

struct GameEntry {
    id: String,
    room: Mutex<Room>,
    seats: Mutex<[Option<Seat>; 2]>,
    /// Open sockets per seat (a player may have several tabs).
    connections: Mutex<[u32; 2]>,
    abandon_after: Duration,
    /// Fan-out of broadcast messages to every connection on this game.
    tx: broadcast::Sender<ServerMessage>,
    db: Option<Db>,
    rated: bool,
    /// Set once a rated game's ratings have been updated.
    rating_diffs: Mutex<Option<RatingDiffs>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CreateGame {
    /// Milliseconds per side; default 5 minutes.
    pub initial_ms: Option<u64>,
    /// Milliseconds added after each move; default 0.
    pub increment_ms: Option<u64>,
    /// Whether the result changes ratings; needs an account. Default casual.
    #[serde(default)]
    pub rated: bool,
}

#[derive(Debug)]
pub enum CreateError {
    /// Rated games are for registered accounts.
    NeedsAccount,
    Db(sqlx::Error),
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
    pub rated: bool,
    /// For rated games, once the ratings have been updated.
    pub rating_diffs: Option<RatingDiffs>,
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
    /// A rated game, and the caller is a guest.
    NeedsAccount,
    Db(sqlx::Error),
}

impl Games {
    /// Games that are also written to Postgres.
    pub fn with_db(db: Db) -> Games {
        Games {
            db: Some(db),
            ..Games::default()
        }
    }

    /// Change how long a disconnected player has to come back.
    pub fn abandon_after(mut self, grace: Duration) -> Games {
        self.abandon_after = grace;
        self
    }

    /// A seat for `user`, with their rating in `category` if they have an account.
    async fn seat(&self, user: &User, category: Category) -> Result<Seat, sqlx::Error> {
        let rating = match (&self.db, user.is_guest) {
            (Some(db), false) => {
                let rating = db.rating(&user.id, category).await?;
                Some(PlayerRating {
                    value: rating.shown(),
                    provisional: rating.provisional(),
                })
            }
            _ => None,
        };
        Ok(Seat {
            user_id: user.id.clone(),
            username: user.username.clone(),
            rating,
        })
    }

    /// Create a game with `white` in the White seat.
    pub async fn create(
        &self,
        white: &User,
        time_control: TimeControl,
        rated: bool,
    ) -> Result<CreatedGame, CreateError> {
        if rated && white.is_guest {
            return Err(CreateError::NeedsAccount);
        }
        let id = uuid::Uuid::new_v4().to_string();
        let room = Room::new(time_control);
        let seat = self
            .seat(white, room.category())
            .await
            .map_err(CreateError::Db)?;
        let seats = [Some(seat), None];
        if let Some(db) = &self.db {
            db.insert(&id, &seats, time_control, rated)
                .await
                .map_err(CreateError::Db)?;
        }
        self.insert(&id, seats, room, rated, None);
        Ok(CreatedGame { id })
    }

    /// Take the open Black seat.
    pub async fn join(&self, id: &str, user: &User) -> Result<Players, JoinError> {
        let entry = self.get(id).await.ok_or(JoinError::NotFound)?;
        if entry.rated && user.is_guest {
            return Err(JoinError::NeedsAccount);
        }
        let category = entry.room.lock().unwrap().category();
        let seat = self.seat(user, category).await.map_err(JoinError::Db)?;
        let players = {
            let mut seats = entry.seats.lock().unwrap();
            if seats[0].as_ref().is_some_and(|s| s.user_id == user.id) {
                return Err(JoinError::OwnGame);
            }
            match &seats[1] {
                // Already seated: nothing changes, nobody needs telling.
                Some(s) if s.user_id == user.id => return Ok(players_of(&seats)),
                Some(_) => return Err(JoinError::SeatTaken),
                None => seats[1] = Some(seat.clone()),
            }
            players_of(&seats)
        };
        if let Some(db) = &self.db {
            db.set_black(id, &seat).await.map_err(JoinError::Db)?;
        }
        let _ = entry.tx.send(ServerMessage::PlayersChanged {
            players: players.clone(),
        });
        Ok(players)
    }

    fn insert(
        &self,
        id: &str,
        seats: [Option<Seat>; 2],
        room: Room,
        rated: bool,
        rating_diffs: Option<RatingDiffs>,
    ) -> Arc<GameEntry> {
        let (tx, _) = broadcast::channel(64);
        let entry = Arc::new(GameEntry {
            id: id.to_string(),
            room: Mutex::new(room),
            seats: Mutex::new(seats),
            connections: Mutex::new([0, 0]),
            abandon_after: self.abandon_after,
            tx,
            db: self.db.clone(),
            rated,
            rating_diffs: Mutex::new(rating_diffs),
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
        let over = room.is_over();
        let entry = self.insert(id, stored.seats, room, stored.rated, stored.rating_diffs);
        // A restored game may already be past its deadline, or have ended
        // without its ratings applied (the server stopped in between).
        entry.arm_flag_timer();
        if over {
            entry.apply_ratings().await;
        }
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
    /// Which seat `user` holds. The seat's name is refreshed from `user`
    /// while we're at it: a guest who signed up mid-game comes back with a
    /// username, and everyone watching hears about it.
    fn side_of(&self, user: Option<&User>) -> Option<Color> {
        let user = user?;
        let (side, renamed) = {
            let mut seats = self.seats.lock().unwrap();
            let (index, side) = if seats[0].as_ref().is_some_and(|s| s.user_id == user.id) {
                (0, Color::White)
            } else if seats[1].as_ref().is_some_and(|s| s.user_id == user.id) {
                (1, Color::Black)
            } else {
                return None;
            };
            let seat = seats[index].as_mut().expect("checked above");
            let renamed = seat.username != user.username;
            if renamed {
                seat.username = user.username.clone();
            }
            (side, renamed.then(|| players_of(&seats)))
        };
        if let Some(players) = renamed {
            let _ = self.tx.send(ServerMessage::PlayersChanged { players });
        }
        Some(side)
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
            return;
        }
        if snapshot.ended.is_some() {
            self.apply_ratings().await;
        }
    }

    /// For a finished rated game, update the players' ratings (once; the
    /// database makes repeats no-ops) and tell everyone watching.
    async fn apply_ratings(&self) {
        let Some(db) = &self.db else { return };
        if !self.rated {
            return;
        }
        match db.apply_ratings(&self.id).await {
            Ok(Some(diffs)) => {
                *self.rating_diffs.lock().unwrap() = Some(diffs);
                let _ = self.tx.send(ServerMessage::RatingsChanged { diffs });
            }
            Ok(None) => {}
            Err(e) => tracing::error!("rating game {}: {e}", self.id),
        }
    }

    /// A socket for `side` opened (`true`) or closed. Recomputes who is
    /// away and arms the abandonment timer.
    fn attendance(self: &Arc<Self>, side: Color, arrived: bool) {
        let connected = {
            let mut counts = self.connections.lock().unwrap();
            let i = if side.is_white() { 0 } else { 1 };
            counts[i] = if arrived {
                counts[i] + 1
            } else {
                counts[i].saturating_sub(1)
            };
            counts.map(|c| c > 0)
        };
        let seated = self.seats.lock().unwrap().each_ref().map(Option::is_some);
        let out = self.room.lock().unwrap().set_presence(
            connected,
            seated,
            self.abandon_after,
            Instant::now(),
        );
        if let Some(out) = out {
            self.dispatch(vec![out], &mut Vec::new());
        }
        self.arm_abandon_timer();
    }

    /// Wake up when the away player's countdown ends, and end the game then
    /// if they are still away. Stale timers find nothing to do.
    fn arm_abandon_timer(self: &Arc<Self>) {
        let Some(deadline) = self.room.lock().unwrap().away_deadline() else {
            return;
        };
        let entry = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep_until((deadline + Duration::from_millis(1)).into()).await;
            let out = entry.room.lock().unwrap().check_abandonment(Instant::now());
            if let Some(out) = out
                && entry.dispatch(vec![out], &mut Vec::new())
            {
                entry.persist().await;
            }
        });
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
    match games.create(&user, tc, body.rated).await {
        Ok(created) => (StatusCode::CREATED, Json(created)).into_response(),
        Err(CreateError::NeedsAccount) => (
            StatusCode::FORBIDDEN,
            "rated games need an account: sign up or log in",
        )
            .into_response(),
        Err(CreateError::Db(e)) => {
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
        Err(JoinError::NeedsAccount) => (
            StatusCode::FORBIDDEN,
            "this is a rated game: sign up or log in to play it",
        )
            .into_response(),
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

/// A seated player's open socket, counted while it lives. Dropping it (the
/// session ending for any reason) counts it out.
struct Attendance {
    entry: Arc<GameEntry>,
    side: Color,
}

impl Attendance {
    fn start(entry: &Arc<GameEntry>, side: Color) -> Attendance {
        entry.attendance(side, true);
        Attendance {
            entry: Arc::clone(entry),
            side,
        }
    }
}

impl Drop for Attendance {
    fn drop(&mut self) {
        self.entry.attendance(self.side, false);
    }
}

async fn session(socket: WebSocket, entry: Arc<GameEntry>, side: Option<Color>) {
    let (mut sink, mut stream) = socket.split();
    let mut rx = entry.tx.subscribe();
    // Before the Sync, so it already reflects that this player is back.
    let _attendance = side.map(|side| Attendance::start(&entry, side));

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
                away,
                category,
                ..
            } => ServerMessage::Sync {
                start_fen,
                moves,
                clocks,
                your_color,
                ended,
                draw_offer,
                players,
                away,
                rated: self.rated,
                category,
                rating_diffs: *self.rating_diffs.lock().unwrap(),
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
