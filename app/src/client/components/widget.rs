use chess::{Color, Player, PlayerKind};
use common::theme::ThemeType;
use dioxus::prelude::*;

use super::{
    super::{
        components::BoardButtons,
        shared_states::{Analyze, BoardSize},
    },
    settings::{load_theme_from_config, Settings},
    Board, EvalBar, InfoBar,
};

#[component]
pub(crate) fn Widget(cx: Scope) -> Element {
    let board_theme = use_state(cx, || load_theme_from_config(ThemeType::Board));
    let piece_theme = use_state(cx, || load_theme_from_config(ThemeType::Piece));
    let analyze = **use_shared_state::<Analyze>(cx)?.read();
    let board_size = **use_shared_state::<BoardSize>(cx)?.read();
    let white_player = use_lock(cx, || Player::with_color(Color::White));
    let black_player = use_lock(cx, || Player::with_color(Color::Black));
    let white_player_kind = white_player.read().kind;
    let black_player_kind = black_player.read().kind;

    cx.render(rsx! {
        div { class: "widget-container", style: "height: {board_size}px",
            div {
                Board {
                    white_player_kind: white_player_kind,
                    black_player_kind: black_player_kind,
                    board_theme: board_theme.to_string(),
                    piece_theme: piece_theme.to_string(),
                }
                BoardButtons {
                    white_player: white_player.to_owned(),
                    black_player: black_player.to_owned(),
                }
            }
            if analyze {
                rsx! { EvalBar {} }
            },
            InfoBar {
                is_local_game: PlayerKind::is_local_game(white_player_kind, black_player_kind)
            },
            Settings {
                board_theme: board_theme.to_owned(),
                piece_theme: piece_theme.to_owned(),
            },
        }
    })
}
