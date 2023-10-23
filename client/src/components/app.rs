use super::Widget;
use crate::shared_states::GameId;
use std::time::Duration;

use chess::game::Game;
use chess::{
    color::Color,
    player::{Player, PlayerKind},
};
use dioxus::prelude::*;
use server_functions::setup_remote_game;
const WIDGET_HEIGHT: u32 = 800;
const START_TIME: Duration = Duration::from_secs(3600);

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

pub(crate) fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, || GameId(None));
    use_shared_state_provider(cx, || Game::with_start_time(START_TIME));

    #[cfg(not(target_arch = "wasm32"))]
    let window = dioxus_desktop::use_window(cx);

    let white_player = use_lock(cx, || Player::with_color(Color::White));
    let black_player = use_lock(cx, || Player::with_color(Color::Black));
    let perspective = use_state(cx, || Color::White);
    let game = use_shared_state::<Game>(cx).unwrap();
    let game_id = use_shared_state::<GameId>(cx).unwrap();
    let analyze = use_state(cx, || false);

    cx.render(rsx! {
        style { include_str!("../../styles/output.css") }
        Widget {
            white_player: white_player.to_owned(),
            black_player: black_player.to_owned(),
            perspective: *perspective.get(),
            analyze: analyze.to_owned(),
            start_time: START_TIME,
            height: WIDGET_HEIGHT
        }
        div {
            class: "flex justify-center items-center",
            style: "width: {WIDGET_HEIGHT}px",
            button {
                class: "button",
                style: "top: {WIDGET_HEIGHT}px",
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
                                perspective.set(get_default_perspective(&white_player, &black_player));
                                analyze.set(false);
                            }
                            Err(err) => log::error!("Error starting remote game: {err:?}"),
                        }
                    })
                },
                "Play Remote"
            }
            button {
                class: "button",
                style: "top: {WIDGET_HEIGHT}px",
                onclick: |_| perspective.modify(|perspective| !*perspective),
                "Flip Board"
            }
            button {
                class: "button",
                style: "top: {WIDGET_HEIGHT}px",
                hidden: !game.read().game_over()
                    && (white_player.read().kind != PlayerKind::Local
                        || black_player.read().kind != PlayerKind::Local),
                onclick: |_| analyze.modify(|analyze| !*analyze),
                if **analyze { "Stop analyzing" } else { "Analyze" }
            }
            {
                #[cfg(not(target_arch = "wasm32"))]
                rsx! {
                    button {
                        class: "button",
                        style: "top: {WIDGET_HEIGHT}px",
                        onclick: |_| {
                            log::info!("Quitting game..");
                            window.close()
                        },
                        "Quit"
                    }
                }
            }
        }
    })
}
