#[cfg(feature = "server")]
pub mod games;
mod get_themes;
mod setup_remote_game;

pub(crate) use get_themes::get_themes;
pub(crate) use setup_remote_game::setup_remote_game;
