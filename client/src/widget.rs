use crate::board::Board;
use crate::info_bar::InfoBar;

use chess::game::Game;
use chess::player::Player;
use dioxus::prelude::*;
use std::time::Duration;

const BOARD_SIZE: u32 = 800;

#[derive(Props, PartialEq)]
pub struct WidgetProps {
    white_player: Player,
    black_player: Player,
    time: Duration,
}

pub fn Widget(cx: Scope<WidgetProps>) -> Element {
    let game = use_ref(cx, || Game::builder().duration(cx.props.time).build());
    cx.render(rsx! {
        Board {
            size: BOARD_SIZE,
            game: game,
            white_player_kind: cx.props.white_player.kind,
            black_player_kind: cx.props.black_player.kind,
        },
        InfoBar {
            game: game,
            time: cx.props.time,
            left: BOARD_SIZE,
        }
    })
}
