use super::Board;
use super::EvalBar;
use super::InfoBar;

use chess::color::Color;
use chess::player::Player;
use dioxus::prelude::*;
use std::fs;
use std::io;
use std::time::Duration;

#[component]
pub(crate) fn Widget(
    cx: Scope,
    white_player: UseLock<Player>,
    black_player: UseLock<Player>,
    perspective: Color,
    analyze: UseState<bool>,
    start_time: Duration,
    height: u32,
) -> Element {
    let board_theme = use_state(cx, || String::from("qootee"));
    let piece_theme = use_state(cx, || String::from("maestro"));
    cx.render(rsx! {
        div { class: "widget-container", style: "height: {height}px",
            Board {
                size: *height,
                white_player_kind: white_player.read().kind,
                black_player_kind: black_player.read().kind,
                perspective: *perspective,
                analyze: analyze.to_owned(),
            }
            if **analyze {
                rsx! { EvalBar { perspective: *perspective } }
            }
            InfoBar { start_time: *start_time },
            // Dropdown for selecting board theme
            select {
                onchange: |event| board_theme.set(event.value.clone()),
                get_themes(ThemeType::Board).unwrap().into_iter().map(|theme| {
                    rsx! {
                        option { value: "{theme}", "{theme}" }
                    }
                })
                "Select board theme"
            }
            // Dropdown for selecting piece theme
            select {
                onchange: |event| piece_theme.set(event.value.clone()),
                get_themes(ThemeType::Piece).unwrap().into_iter().map(|theme| {
                    rsx! {
                        option { value: "{theme}", "{theme}" }
                    }
                })
                "Select piece theme"
            }
        }
    })
}

enum ThemeType {
    Board,
    Piece,
}

fn get_themes(theme_type: ThemeType) -> io::Result<Vec<String>> {
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
