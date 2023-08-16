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
    start_time: Duration,
    height: u32,
}

pub fn Widget(cx: Scope<WidgetProps>) -> Element {
    use_shared_state_provider(cx, || Game::with_start_time(cx.props.start_time));
    cx.render(rsx! {
        Board {
            size: cx.props.height,
            game_id: cx.props.game_id,
            white_player_kind: cx.props.white_player.read().kind,
            black_player_kind: cx.props.black_player.read().kind,
            perspective: cx.props.perspective
        }
        InfoBar { start_time: cx.props.start_time, left: cx.props.height }
    })
}
