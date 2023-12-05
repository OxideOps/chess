use dioxus::html::KeyCode::P;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ThemeType {
    Board,
    Piece,
}

impl ThemeType {
    pub(crate) fn default_theme(&self) -> String {
        match self {
            ThemeType::Board => "qootee".into(),
            ThemeType::Piece => "merida".into(),
        }
    }

    pub(crate) fn to_string(&self) -> String {
        match self {
            ThemeType::Board => "board_theme".into(),
            ThemeType::Piece => "piece_theme".into(),
        }
    }
}

#[cfg(not(feature = "web"))]
pub async fn get_themes(theme_type: ThemeType) -> std::io::Result<Vec<String>> {
    let mut themes = Vec::new();
    let dir_path = match theme_type {
        ThemeType::Board => "images/boards/",
        ThemeType::Piece => "images/pieces/",
    };

    for entry in std::fs::read_dir(dir_path)? {
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
