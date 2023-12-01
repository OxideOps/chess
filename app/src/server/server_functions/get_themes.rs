use dioxus_fullstack::prelude::*;

use crate::common::theme;

#[server(GetThemes, "/api")]
pub async fn get_themes(theme_type: theme::ThemeType) -> Result<Vec<String>, ServerFnError> {
    Ok(theme::get_themes(theme_type).await?)
}
