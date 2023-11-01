use serde::{Deserialize, Serialize};
use std::{fs, io};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ThemeType {
    Board,
    Piece,
}

pub async fn get_themes(theme_type: ThemeType) -> io::Result<Vec<String>> {
    let mut themes = Vec::new();
    let mut dir_path = match theme_type {
        ThemeType::Board => "images/boards/".to_string(),
        ThemeType::Piece => "images/pieces/".to_string(),
    };
    if cfg!(feature = "ssr") {
        dir_path.insert_str(0, "dist/");
    }

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
