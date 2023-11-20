use chess::Player;
use common::theme::ThemeType;
use dioxus::prelude::*;

use super::super::shared_states::BoardSize;
use super::{
    // settings::{load_theme_from_config, Settings},
    Board,
    EvalBar,
    InfoBar,
};

#[component]
pub(crate) fn Widget(
    cx: Scope,
    white_player: UseLock<Player>,
    black_player: UseLock<Player>,
    analyze: UseState<bool>,
) -> Element {
    // let board_theme = use_state(cx, || load_theme_from_config(ThemeType::Board));
    // let piece_theme = use_state(cx, || load_theme_from_config(ThemeType::Piece));
    let board_size = **use_shared_state::<BoardSize>(cx)?.read();

    cx.render(rsx! {
        div { class: "widget-container", style: "height: {board_size}px",
            Board {
                white_player_kind: white_player.read().kind,
                black_player_kind: black_player.read().kind,
                analyze: analyze.to_owned(),
                board_theme: "danya".into(),
                piece_theme: "merida".into(),
            },
            if **analyze {
                rsx! { EvalBar {} }
            },
            InfoBar {},
            // Settings {
            //     board_theme: board_theme.to_owned(),
            //     piece_theme: piece_theme.to_owned(),
            // },
        }
    })
}
