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
    use once_cell::sync::Lazy;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    pub static GAMES: Lazy<Arc<RwLock<HashSet<u32>>>> =
        Lazy::new(|| Arc::new(RwLock::new(HashSet::new())));
    pub static PENDING_GAME: Lazy<Arc<RwLock<Option<u32>>>> =
        Lazy::new(|| Arc::new(RwLock::new(None)));
}

#[server(SetupRemoteGame, "/api")]
pub async fn setup_remote_game() -> Result<RemoteGameInfo, ServerFnError> {
    use games::{GAMES, PENDING_GAME};
    use rand::distributions::{Distribution, Uniform};

    let mut games = GAMES.write().await;
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
    while games.contains(&game_id) {
        game_id = range.sample(&mut rng);
    }

    games.insert(game_id);
    *pending_game = Some(game_id);

    Ok(RemoteGameInfo {
        game_id,
        local_color: Color::White,
    })
}
