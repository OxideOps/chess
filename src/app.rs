use crate::chess_widget::ChessWidget;
use crate::pieces::Color;
use crate::player::Player;

use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    render! {
        ChessWidget {
            white_player: Player::with_color(Color::White),
            black_player: Player::with_color(Color::Black),
        },
    }
}
