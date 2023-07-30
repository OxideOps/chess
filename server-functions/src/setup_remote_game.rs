use dioxus_fullstack::prelude::*;

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

#[server(SetupRemoteGame)]
async fn setup_remote_game() -> Result<u32, ServerFnError> {
    use games::{GAMES, PENDING_GAME};
    use rand::distributions::{Distribution, Uniform};

    if let Some(pending_game) = *PENDING_GAME.read().await {
        *PENDING_GAME.write().await = None;
        return Ok(pending_game);
    }

    let mut rng = rand::thread_rng();
    let range = Uniform::from(1..10000000);
    let mut game_id = 0;
    while GAMES.read().await.contains(&game_id) {
        game_id = range.sample(&mut rng);
    }

    GAMES.write().await.insert(game_id);
    *PENDING_GAME.write().await = Some(game_id);
    Ok(game_id)
}
