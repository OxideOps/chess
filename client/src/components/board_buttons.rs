use chess::{Color, Game, Player, PlayerKind};
use dioxus::prelude::*;
use server_functions::setup_remote_game;

use crate::shared_states::{Analyze, BoardSize, GameId, Perspective};

#[component]
pub(crate) fn BoardButtons(
    cx: Scope,
    white_player: UseLock<Player>,
    black_player: UseLock<Player>,
) -> Element {
    let analyze = use_shared_state::<Analyze>(cx)?;
    let board_size = **use_shared_state::<BoardSize>(cx)?.read();
    let perspective = use_shared_state::<Perspective>(cx)?;
    let game = use_shared_state::<Game>(cx)?;
    let game_id = use_shared_state::<GameId>(cx)?;

    cx.render(rsx! {
        div { class: "board-buttons-container", style: "width: {board_size}px",
            button { class: "button",
                onclick: |_| {
                    to_owned![analyze, white_player, black_player, perspective, game, game_id];
                    cx.spawn(async move {
                        match setup_remote_game().await {
                            Ok(info) => {
                                log::info!("Setting up remote game: {info:?}");
                                game.write().reset();
                                **game_id.write() = Some(info.game_id);
                                let player = match info.local_color {
                                    Color::White => black_player.to_owned(),
                                    Color::Black => white_player.to_owned(),
                                };
                                player.write().kind = PlayerKind::Remote;
                                **perspective.write() = get_default_perspective(&white_player, &black_player);
                                **analyze.write() = false;
                            }
                            Err(err) => log::error!("Error starting remote game: {err:?}"),
                        }
                    })
                },
                "Play Remote"
            }
            button { class: "button",
                onclick: |_| perspective.with_mut(|perspective| **perspective = !**perspective),
                "Flip Board"
            }
            button { class: "button",
                hidden: !game.read().game_over()
                    && (white_player.read().kind != PlayerKind::Local
                        || black_player.read().kind != PlayerKind::Local),
                onclick: |_| analyze.with_mut(|analyze| **analyze = !**analyze),
                if **analyze.read() { "Stop analyzing" } else { "Analyze" }
            }
            {
                #[cfg(not(target_arch = "wasm32"))]
                rsx! {
                    button {
                        class: "button",
                        onclick: |_| {
                            log::info!("Quitting game..");
                            dioxus_desktop::use_window(cx).close()
                        },
                        "Quit"
                    }
                }
            }
        }
    })
}

fn get_default_perspective(
    white_player: &UseLock<Player>,
    black_player: &UseLock<Player>,
) -> Color {
    if black_player.read().kind == PlayerKind::Local
        && white_player.read().kind != PlayerKind::Local
    {
        Color::Black
    } else {
        Color::White
    }
}
