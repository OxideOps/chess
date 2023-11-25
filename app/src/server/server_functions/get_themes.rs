use common::theme;
use dioxus_fullstack::prelude::*;

#[server(GetThemes, "/api")]
pub async fn get_themes(theme_type: theme::ThemeType) -> Result<Vec<String>, ServerFnError> {
    Ok(theme::get_themes(theme_type).await?)
}
