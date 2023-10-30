mod setup_remote_game;

#[cfg(feature = "ssr")]
pub use setup_remote_game::games::*;
pub use setup_remote_game::*;

use dioxus_fullstack::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ThemeType {
    Board,
    Piece,
}

#[server(GetThemes, "/api")]
pub async fn get_themes(theme_type: ThemeType) -> Result<Vec<String>, ServerFnError> {
    let mut themes = Vec::new();
    let dir_path = match theme_type {
        ThemeType::Board => "images/boards/",
        ThemeType::Piece => "images/pieces/",
    };

    for entry in fs::read_dir(dir_path)? {
        let path = entry?.path();

        if path.is_dir() {
            if let Some(theme_name) = path.file_name() {
                if let Some(theme_str) = theme_name.to_str() {
                    themes.push(theme_str.to_string());
                }
            }
        }
    }

    Ok(themes)
}
