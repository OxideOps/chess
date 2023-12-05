use std::fmt::Formatter;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ThemeType {
    Board,
    Piece,
}

impl std::fmt::Display for ThemeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::Board => "board_theme",
            Self::Piece => "piece_theme",
        };
        write!(f, "{str}")
    }
}

impl ThemeType {
    pub fn default_theme(&self) -> String {
        match self {
            Self::Board => "qootee".to_string(),
            Self::Piece => "merida".to_string(),
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
