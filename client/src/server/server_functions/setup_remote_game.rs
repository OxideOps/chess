use chess::Color;
use dioxus_fullstack::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteGameInfo {
    pub game_id: u32,
    pub local_color: Color,
}

#[server(SetupRemoteGame, "/api")]
pub async fn setup_remote_game() -> Result<RemoteGameInfo, ServerFnError> {
    use rand::distributions::{Distribution, Uniform};

    use super::games::{PlayerConnections, GAMES, PENDING_GAME};

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
