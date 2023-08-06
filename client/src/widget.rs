use crate::board::Board;
use crate::info_bar::InfoBar;

use chess::color::Color;
use chess::game::Game;
use chess::player::Player;
use dioxus::prelude::*;
use std::time::Duration;

#[derive(Props, PartialEq)]
pub struct WidgetProps {
    #[props(!optional)]
    game_id: Option<u32>,
    white_player: UseRef<Player>,
    black_player: UseRef<Player>,
    perspective: Color,
    time: Duration,
    height: u32,
}

pub fn Widget(cx: Scope<WidgetProps>) -> Element {
    let game = use_ref(cx, || Game::builder().duration(cx.props.time).build());
    cx.render(rsx! {
        Board {
            size: cx.props.height,
            game: game,
            game_id: cx.props.game_id,
            white_player_kind: cx.props.white_player.with(|player| player.kind),
            black_player_kind: cx.props.black_player.with(|player| player.kind),
            perspective: cx.props.perspective
        }
        InfoBar { game: game, time: cx.props.time, left: cx.props.height }
    })
}
