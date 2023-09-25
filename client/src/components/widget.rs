use crate::components::Board;
use crate::components::InfoBar;

use chess::color::Color;
use chess::player::Player;
use dioxus::prelude::*;
use std::time::Duration;

#[derive(Props, PartialEq)]
pub(crate) struct WidgetProps {
    white_player: UseLock<Player>,
    black_player: UseLock<Player>,
    perspective: Color,
    analyze: UseState<bool>,
    start_time: Duration,
    height: u32,
}

pub(crate) fn Widget(cx: Scope<WidgetProps>) -> Element {
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
