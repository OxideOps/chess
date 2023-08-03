use chess::color::Color;
use dioxus_fullstack::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteGameInfo {
    pub game_id: u32,
    pub local_color: Color,
}

#[cfg(feature = "ssr")]
pub mod games {
    use axum::extract::ws::{Message, WebSocket};
    use futures::stream::{SplitSink, SplitStream};
    use once_cell::sync::Lazy;
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::{Mutex, RwLock};

    pub type WriteStream = Arc<Mutex<Option<SplitSink<WebSocket, Message>>>>;
    pub type ReadStream = Arc<Mutex<Option<SplitStream<WebSocket>>>>;
    pub type PlayerConnections = Arc<[(WriteStream, ReadStream); 2]>;

    pub static GAMES: Lazy<Arc<Mutex<HashMap<u32, PlayerConnections>>>> =
        Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
    pub static PENDING_GAME: Lazy<Arc<RwLock<Option<u32>>>> =
        Lazy::new(|| Arc::new(RwLock::new(None)));
}

#[server(SetupRemoteGame, "/api")]
pub async fn setup_remote_game() -> Result<RemoteGameInfo, ServerFnError> {
    use games::{PlayerConnections, GAMES, PENDING_GAME};
    use rand::distributions::{Distribution, Uniform};

    let mut games = GAMES.lock().await;
    let mut pending_game = PENDING_GAME.write().await;
    if let Some(game_id) = *pending_game {
        *pending_game = None;
        return Ok(RemoteGameInfo {
            game_id,
            local_color: Color::Black,
        });
    }

    let mut rng = rand::thread_rng();
    let range = Uniform::from(1..10000000);
    let mut game_id = 0;
    while games.contains_key(&game_id) {
        game_id = range.sample(&mut rng);
    }

    games.insert(game_id, PlayerConnections::default());
    *pending_game = Some(game_id);

    Ok(RemoteGameInfo {
        game_id,
        local_color: Color::White,
    })
}
