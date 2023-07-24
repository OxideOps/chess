use crate::chess_widget::Widget;

use chess::{player::Player, pieces::Color};
use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        Widget {
            white_player: Player::with_color(Color::White),
            black_player: Player::with_color(Color::Black),
        },
    })
}
