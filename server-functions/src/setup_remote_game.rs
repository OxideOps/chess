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
    use std::{collections::HashMap, sync::Arc};

    use axum::extract::ws::{Message, WebSocket};
    use futures::stream::{SplitSink, SplitStream};
    use once_cell::sync::Lazy;
    use tokio::sync::{Mutex, RwLock};

    pub type Send = Arc<Mutex<SplitSink<WebSocket, Message>>>;
    pub type Recv = Arc<Mutex<SplitStream<WebSocket>>>;
    pub type PlayerConnections = Arc<Mutex<Vec<(Send, Recv)>>>;

    pub static GAMES: Lazy<Arc<RwLock<HashMap<u32, PlayerConnections>>>> =
        Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
    pub static PENDING_GAME: Lazy<Arc<Mutex<Option<u32>>>> =
        Lazy::new(|| Arc::new(Mutex::new(None)));
}

#[server(SetupRemoteGame, "/api")]
pub async fn setup_remote_game() -> Result<RemoteGameInfo, ServerFnError> {
    use games::{PlayerConnections, GAMES, PENDING_GAME};
    use rand::distributions::{Distribution, Uniform};

    let mut games = GAMES.write().await;
    let mut pending_game = PENDING_GAME.lock().await;
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
