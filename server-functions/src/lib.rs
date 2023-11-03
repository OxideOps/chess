mod setup_remote_game;

use common::theme;
use dioxus_fullstack::prelude::*;
#[cfg(feature = "ssr")]
pub use setup_remote_game::games::*;
pub use setup_remote_game::*;

#[server(GetThemes, "/api")]
pub async fn get_themes(theme_type: theme::ThemeType) -> Result<Vec<String>, ServerFnError> {
    Ok(theme::get_themes(theme_type).await?)
}
