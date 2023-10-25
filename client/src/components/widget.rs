use super::Board;
use super::InfoBar;

use chess::player::Player;
use dioxus::prelude::*;
use std::time::Duration;

#[component]
pub(crate) fn Widget(
    cx: Scope,
    white_player: UseLock<Player>,
    black_player: UseLock<Player>,
    start_time: Duration,
    height: u32,
) -> Element {
    cx.render(rsx! {
        Board {
            size: *height,
            white_player_kind: white_player.read().kind,
            black_player_kind: black_player.read().kind,
        }
        InfoBar { start_time: *start_time, left: *height }
    })
}
