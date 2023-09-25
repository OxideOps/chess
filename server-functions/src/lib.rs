mod setup_remote_game;

#[cfg(feature = "ssr")]
pub use setup_remote_game::games::*;
pub use setup_remote_game::*;
