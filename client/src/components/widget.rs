use chess::{color::Color, player::Player};
use common::theme::ThemeType;
use dioxus::prelude::*;

use super::{
    settings::{load_theme_from_config, Settings},
    Board, EvalBar, InfoBar,
};

#[component]
pub(crate) fn Widget(
    cx: Scope,
    white_player: UseLock<Player>,
    black_player: UseLock<Player>,
    perspective: Color,
    analyze: UseState<bool>,
    height: u32,
) -> Element {
    let board_theme = use_state(cx, || load_theme_from_config(ThemeType::Board));
    let piece_theme = use_state(cx, || load_theme_from_config(ThemeType::Piece));

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
            InfoBar {},
            Settings {
                board_theme: board_theme.to_owned(),
                piece_theme: piece_theme.to_owned(),
            },
        }
    })
}
