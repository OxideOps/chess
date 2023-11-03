use std::time::Duration;

use chess::{color::Color, player::Player};
use common::theme::ThemeType;
use dioxus::prelude::*;

use super::{Board, EvalBar, InfoBar, ThemeSelect};

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
    let board_theme = use_state(cx, || String::from("qootee"));
    let piece_theme = use_state(cx, || String::from("maestro"));

    cx.render(rsx! {
        div { class: "widget-container", style: "height: {height}px",
            Board {
                size: *height,
                white_player_kind: white_player.read().kind,
                black_player_kind: black_player.read().kind,
                perspective: *perspective,
                analyze: analyze.to_owned(),
                board_theme: board_theme.to_string(),
                piece_theme: piece_theme.to_string(),
            },
            if **analyze {
                rsx! { EvalBar { perspective: *perspective } }
            },
            InfoBar { start_time: *start_time },
            ThemeSelect {
                board_theme: board_theme.to_owned(),
                piece_theme: piece_theme.to_owned(),
            },
        }
    })
}
