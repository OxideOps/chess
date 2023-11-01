use super::Board;
use super::EvalBar;
use super::InfoBar;

use chess::color::Color;
use chess::player::Player;
use common::theme::{get_themes, ThemeType};
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
    let board_theme_list;
    let piece_theme_list;

    #[cfg(target_arch = "wasm32")]
    {
        use server_functions::get_themes as get_themes_server;
        board_theme_list = use_future(cx, (), |_| async {
            get_themes_server(ThemeType::Board).await
        });
        piece_theme_list = use_future(cx, (), |_| async {
            get_themes_server(ThemeType::Piece).await
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        board_theme_list = use_future(cx, (), |_| async { get_themes(ThemeType::Board).await });
        piece_theme_list = use_future(cx, (), |_| async { get_themes(ThemeType::Piece).await });
    }

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
            }
            if **analyze {
                rsx! { EvalBar { perspective: *perspective } }
            }
            InfoBar { start_time: *start_time },
            // Theme selection
            div {
                div {
                    label { "Board theme: " }
                    select {
                        class: "select",
                        onchange: |event| board_theme.set(event.value.clone()),
                        if let Some(list) = board_theme_list.value() {
                            rsx! {
                                list.as_ref().unwrap().iter().map(|theme| {
                                    rsx! {
                                        option { value: "{theme}", "{theme}" }
                                    }
                                })
                            }
                        },
                    }
                }
                div {
                    label { "Piece theme: " }
                    select {
                        class: "select",
                        onchange: |event| piece_theme.set(event.value.clone()),
                        if let Some(list) = piece_theme_list.value() {
                            rsx! {
                                list.as_ref().unwrap().iter().map(|theme| {
                                    rsx! {
                                        option { value: "{theme}", "{theme}" }
                                    }
                                })
                            }
                        },
                    }
                }
            }
        }
    })
}
