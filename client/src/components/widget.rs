use super::Board;
use super::EvalBar;
use super::InfoBar;

use chess::color::Color;
use chess::player::Player;
use dioxus::prelude::*;
use std::time::Duration;

#[component]
pub(crate) fn Widget(
    cx: Scope,
    white_player: UseLock<Player>,
    black_player: UseLock<Player>,
    perspective: Color,
    analyze: UseState<bool>,
    start_time: Duration,
    height: u32,
) -> Element {
    cx.render(rsx! {
        div { class: "widget-container", style: "height: {height}px",
            Board {
                size: *height,
                white_player_kind: white_player.read().kind,
                black_player_kind: black_player.read().kind,
                perspective: *perspective,
                analyze: analyze.to_owned()
            }
            if **analyze {
                rsx! { EvalBar { perspective: *perspective } }
            }
            InfoBar { start_time: *start_time }
        }
    })
}
