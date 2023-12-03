mod accounts;
#[cfg(feature = "ssr")]
pub mod games;
mod get_themes;
mod setup_remote_game;

#[cfg(feature = "web")]
pub(crate) use get_themes::get_themes;
#[cfg(not(feature = "ssr"))]
pub(crate) use setup_remote_game::setup_remote_game;
