//! The lobby: open seeks, and the socket that shows them.
//!
//! `GET /api/lobby/ws` is the only endpoint. A connection is told every open
//! seek when it arrives and again whenever the list changes, can post one
//! seek of its own, cancel it, or accept someone else's.
//!
//! A seek is not a game. The game is created at the moment someone accepts,
//! so nobody waits at a half-empty board, and colours are drawn then.
//!
//! **Seeks live as long as the connection that posted them.** They are not
//! written to the database and they do not survive a restart, which is the
//! behaviour we want rather than a limitation: a seek whose author has
//! closed their laptop is one nobody can play, and a lobby full of those is
//! what makes a small site feel abandoned. Closing the socket withdraws the
//! seek; so does taking a game.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use chess_core::{
    Color,
    protocol::{Category, LobbyClientMessage, LobbyServerMessage, SeekInfo},
};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::sync::broadcast;

use crate::{
    AppState,
    auth::{CurrentUser, User},
    games::{CreateError, Games},
    room::TimeControl,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/lobby/ws", get(connect))
}

/// How many messages a slow connection may fall behind before it is dropped.
const BACKLOG: usize = 64;

/// A seek and who is waiting on it.
#[derive(Debug, Clone)]
struct Seek {
    info: SeekInfo,
    /// Who is waiting. Kept whole so the game can be created without
    /// looking anyone up again.
    user: User,
    /// The connection that posted it; the seek dies with it.
    connection: u64,
    time_control: TimeControl,
}

/// What every lobby connection hears about.
#[derive(Debug, Clone)]
enum Event {
    /// The list changed; everyone redraws.
    Changed,
    /// Two people were matched. Each connection checks whether it is one of
    /// them, so nobody learns about anyone else's game.
    Matched {
        game_id: String,
        white: Seated,
        black: Seated,
    },
}

/// Who took a seat: the id the connection matches itself against, and the
/// name to show the other player.
#[derive(Debug, Clone)]
struct Seated {
    id: String,
    username: Option<String>,
}

impl From<&User> for Seated {
    fn from(user: &User) -> Seated {
        Seated {
            id: user.id.clone(),
            username: user.username.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Lobby {
    seeks: Arc<Mutex<HashMap<String, Seek>>>,
    games: Games,
    tx: broadcast::Sender<Event>,
    connections: Arc<AtomicU64>,
}

#[derive(Debug)]
pub enum SeekError {
    /// Rated play needs an account, as it does everywhere else.
    NeedsAccount,
    /// Gone: cancelled, or someone else took it first.
    NotFound,
    /// You can't play yourself.
    YourOwn,
    Db(sqlx::Error),
}

impl SeekError {
    fn message(&self) -> String {
        match self {
            SeekError::NeedsAccount => "rated games need an account: sign up or log in".to_string(),
            SeekError::NotFound => "that seek is no longer open".to_string(),
            SeekError::YourOwn => "that is your own seek".to_string(),
            SeekError::Db(_) => "the server could not start the game".to_string(),
        }
    }
}

impl Lobby {
    pub fn new(games: Games) -> Lobby {
        Lobby {
            seeks: Arc::default(),
            games,
            tx: broadcast::channel(BACKLOG).0,
            connections: Arc::default(),
        }
    }

    /// Every open seek, newest last so the list doesn't jump around.
    pub fn open(&self) -> Vec<SeekInfo> {
        let seeks = self.seeks.lock().unwrap();
        let mut open: Vec<&Seek> = seeks.values().collect();
        open.sort_by(|a, b| a.info.id.cmp(&b.info.id));
        open.iter().map(|s| s.info.clone()).collect()
    }

    /// Post a seek for `user`, replacing whatever `connection` posted before.
    pub async fn post(
        &self,
        user: &User,
        connection: u64,
        time_control: TimeControl,
        rated: bool,
    ) -> Result<SeekInfo, SeekError> {
        if rated && user.is_guest {
            return Err(SeekError::NeedsAccount);
        }
        let category = Category::of(
            time_control.initial.as_millis() as u64,
            time_control.increment.as_millis() as u64,
        );
        let rating = self
            .games
            .rating_for(user, category)
            .await
            .map_err(SeekError::Db)?;
        let info = SeekInfo {
            id: uuid::Uuid::new_v4().to_string(),
            username: user.username.clone(),
            rating,
            initial_ms: time_control.initial.as_millis() as u64,
            increment_ms: time_control.increment.as_millis() as u64,
            rated,
            category,
        };
        {
            let mut seeks = self.seeks.lock().unwrap();
            seeks.retain(|_, s| s.connection != connection);
            seeks.insert(
                info.id.clone(),
                Seek {
                    info: info.clone(),
                    user: user.clone(),
                    connection,
                    time_control,
                },
            );
        }
        let _ = self.tx.send(Event::Changed);
        Ok(info)
    }

    /// Withdraw whatever `connection` posted. Quiet if it had nothing.
    pub fn cancel(&self, connection: u64) {
        let removed = {
            let mut seeks = self.seeks.lock().unwrap();
            let before = seeks.len();
            seeks.retain(|_, s| s.connection != connection);
            seeks.len() != before
        };
        if removed {
            let _ = self.tx.send(Event::Changed);
        }
    }

    /// Take a seek: creates the game and tells both players.
    ///
    /// The seek is removed under the lock before anything else happens, so
    /// two people accepting at the same moment cannot both get a game — the
    /// loser finds it already gone.
    pub async fn accept(&self, id: &str, user: &User) -> Result<(String, Color), SeekError> {
        let seek = {
            let mut seeks = self.seeks.lock().unwrap();
            let seek = seeks.get(id).ok_or(SeekError::NotFound)?;
            if seek.user.id == user.id {
                return Err(SeekError::YourOwn);
            }
            if seek.info.rated && user.is_guest {
                return Err(SeekError::NeedsAccount);
            }
            seeks
                .remove(id)
                .expect("checked above, still under the lock")
        };
        let _ = self.tx.send(Event::Changed);

        let created = self
            .games
            .create_between(&seek.user, user, seek.time_control, seek.info.rated)
            .await;
        let (game_id, poster_color) = match created {
            Ok(pair) => pair,
            Err(CreateError::NeedsAccount) => return Err(SeekError::NeedsAccount),
            Err(CreateError::Db(e)) => return Err(SeekError::Db(e)),
        };
        let (white, black) = if poster_color.is_white() {
            (Seated::from(&seek.user), Seated::from(user))
        } else {
            (Seated::from(user), Seated::from(&seek.user))
        };
        let _ = self.tx.send(Event::Matched {
            game_id: game_id.clone(),
            white,
            black,
        });
        Ok((game_id, !poster_color))
    }

    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    fn next_connection(&self) -> u64 {
        self.connections.fetch_add(1, Ordering::Relaxed)
    }
}

/// The lobby socket. Anyone may watch, including signed-out visitors; a
/// session is only needed to post or accept, and `CurrentUser` makes a guest
/// on demand the same way the rest of the site does.
async fn connect(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !crate::origin::origin_allowed(&headers, &state.config.allowed_origins) {
        return (StatusCode::FORBIDDEN, "cross-origin socket refused").into_response();
    }
    let Some(lobby) = state.lobby.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "no lobby without a database",
        )
            .into_response();
    };
    upgrade.on_upgrade(move |socket| run(socket, lobby, user))
}

async fn run(socket: WebSocket, lobby: Lobby, user: Option<User>) {
    let connection = lobby.next_connection();
    let (mut sink, mut stream) = socket.split();
    let mut events = lobby.subscribe();

    if send(
        &mut sink,
        &LobbyServerMessage::Seeks {
            seeks: lobby.open(),
        },
    )
    .await
    .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(Event::Changed) => {
                    let msg = LobbyServerMessage::Seeks { seeks: lobby.open() };
                    if send(&mut sink, &msg).await.is_err() {
                        break;
                    }
                }
                Ok(Event::Matched { game_id, white, black }) => {
                    let Some(user) = &user else { continue };
                    let (your_color, opponent) = if white.id == user.id {
                        (Color::White, black.username.clone())
                    } else if black.id == user.id {
                        (Color::Black, white.username.clone())
                    } else {
                        continue;
                    };
                    let msg = LobbyServerMessage::GameStarted { game_id, your_color, opponent };
                    if send(&mut sink, &msg).await.is_err() {
                        break;
                    }
                }
                // A connection that fell too far behind gets the current
                // list rather than a gap it can't reason about.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let msg = LobbyServerMessage::Seeks { seeks: lobby.open() };
                    if send(&mut sink, &msg).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = stream.next() => {
                let Some(Ok(message)) = incoming else { break };
                let text = match message {
                    Message::Text(text) => text,
                    Message::Close(_) => break,
                    _ => continue,
                };
                let Ok(request) = serde_json::from_str::<LobbyClientMessage>(&text) else {
                    let msg = LobbyServerMessage::Rejected {
                        message: "unintelligible".to_string(),
                    };
                    if send(&mut sink, &msg).await.is_err() {
                        break;
                    }
                    continue;
                };
                let reply = handle(&lobby, &user, connection, request).await;
                for msg in reply {
                    if send(&mut sink, &msg).await.is_err() {
                        return cleanup(&lobby, connection);
                    }
                }
            }
        }
    }
    cleanup(&lobby, connection);
}

/// The seek goes when the connection does.
fn cleanup(lobby: &Lobby, connection: u64) {
    lobby.cancel(connection);
}

async fn handle(
    lobby: &Lobby,
    user: &Option<User>,
    connection: u64,
    request: LobbyClientMessage,
) -> Vec<LobbyServerMessage> {
    let signed_out = || {
        vec![LobbyServerMessage::Rejected {
            message: "sign in to play".to_string(),
        }]
    };
    match request {
        LobbyClientMessage::Ping => vec![LobbyServerMessage::Pong],
        LobbyClientMessage::CancelSeek => {
            lobby.cancel(connection);
            vec![]
        }
        LobbyClientMessage::PostSeek {
            initial_ms,
            increment_ms,
            rated,
        } => {
            let Some(user) = user else {
                return signed_out();
            };
            let time_control = TimeControl {
                initial: Duration::from_millis(initial_ms),
                increment: Duration::from_millis(increment_ms),
            };
            match lobby.post(user, connection, time_control, rated).await {
                Ok(info) => vec![LobbyServerMessage::SeekPosted { id: info.id }],
                Err(e) => {
                    if let SeekError::Db(e) = &e {
                        tracing::error!("posting a seek: {e}");
                    }
                    vec![LobbyServerMessage::Rejected {
                        message: e.message(),
                    }]
                }
            }
        }
        LobbyClientMessage::AcceptSeek { id } => {
            let Some(user) = user else {
                return signed_out();
            };
            match lobby.accept(&id, user).await {
                // The `Matched` broadcast tells both sides, this one
                // included, so there is nothing to send here.
                Ok(_) => vec![],
                Err(e) => {
                    if let SeekError::Db(e) = &e {
                        tracing::error!("accepting seek {id}: {e}");
                    }
                    vec![LobbyServerMessage::Rejected {
                        message: e.message(),
                    }]
                }
            }
        }
    }
}

async fn send(
    sink: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &LobbyServerMessage,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("protocol types serialize");
    sink.send(Message::Text(text.into())).await
}
