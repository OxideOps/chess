use chess::{Color, Player, PlayerKind};
use dioxus::prelude::*;

use super::{
    super::{
        components::BoardButtons,
        shared_states::{Analyze, BoardSize},
    },
    Board, EvalBar, InfoBar,
};

#[component]
pub(crate) fn Widget(cx: Scope) -> Element {
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
        }
    })
}
