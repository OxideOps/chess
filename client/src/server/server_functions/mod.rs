#[cfg(feature = "server")]
pub mod games;
mod setup_remote_game;

pub(crate) use setup_remote_game::setup_remote_game;
