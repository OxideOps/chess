use crate::chess_widget::{ChessWidget, PlayerType};

use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    render! {
        ChessWidget {
            white_player: PlayerType::Local,
            black_player: PlayerType::Local,
        },
    }
}
