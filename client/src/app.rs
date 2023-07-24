use crate::chess_widget::ChessWidget;
use chess::pieces::Color;
use chess::player::Player;

use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        ChessWidget {
            white_player: Player::with_color(Color::White),
            black_player: Player::with_color(Color::Black),
        },
    })
}
