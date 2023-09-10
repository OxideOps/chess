use crate::board::Board;
use crate::info_bar::InfoBar;

use chess::color::Color;
use chess::game::Game;
use chess::player::Player;
use dioxus::prelude::*;
use std::time::Duration;

#[derive(Props, PartialEq)]
pub struct WidgetProps {
    white_player: UseRef<Player>,
    black_player: UseRef<Player>,
    perspective: Color,
    analyze: UseState<bool>,
    start_time: Duration,
    height: u32,
}

pub fn Widget(cx: Scope<WidgetProps>) -> Element {
    use_shared_state_provider(cx, || Game::with_start_time(cx.props.start_time));
    cx.render(rsx! {
        Board {
            size: cx.props.height,
            white_player_kind: cx.props.white_player.read().kind,
            black_player_kind: cx.props.black_player.read().kind,
            perspective: cx.props.perspective,
            analyze: cx.props.analyze.to_owned()
        }
        InfoBar { start_time: cx.props.start_time, left: cx.props.height }
    })
}
