use crate::widget::Widget;
use std::time::Duration;

use chess::{
    color::Color,
    player::{Player, PlayerKind},
};
use dioxus::prelude::*;
use server_functions::setup_remote_game::setup_remote_game;
const WIDGET_HEIGHT: u32 = 800;

fn get_default_perspective(white_player: &UseRef<Player>, black_player: &UseRef<Player>) -> Color {
    if black_player.with(|player| player.kind == PlayerKind::Local)
        && white_player.with(|player| player.kind != PlayerKind::Local)
    {
        Color::Black
    } else {
        Color::White
    }
}

pub fn App(cx: Scope) -> Element {
    let white_player = use_ref(cx, || Player::with_color(Color::White));
    let black_player = use_ref(cx, || Player::with_color(Color::Black));
    let perspective = use_state(cx, || Color::White);
    let game_id = use_state::<Option<u32>>(cx, || None);
    let analyze = use_state(cx, || false);

    cx.render(rsx! {
        style { include_str!("../../styles/output.css") }
        Widget {
            game_id: *game_id.get(),
            white_player: white_player.to_owned(),
            black_player: black_player.to_owned(),
            perspective: *perspective.get(),
            analyze: analyze.to_owned(),
            start_time: Duration::from_secs(3600),
            height: WIDGET_HEIGHT
        }
        div {
            class: "flex justify-center items-center",
            style: "width: {WIDGET_HEIGHT}px",
            button {
                class: "button",
                style: "top: {WIDGET_HEIGHT}px",
                onclick: |_| {
                    to_owned![white_player, black_player, perspective, game_id];
                    cx.spawn(async move {
                        match setup_remote_game().await {
                            Ok(info) => {
                                log::info!("Setting up remote game: {info:?}");
                                game_id.set(Some(info.game_id));
                                let player = match info.local_color {
                                    Color::White => black_player.to_owned(),
                                    Color::Black => white_player.to_owned(),
                                };
                                player.with_mut(|player| player.kind = PlayerKind::Remote);
                                perspective.set(get_default_perspective(&white_player, &black_player));
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
                onclick: |_| analyze.modify(|analyze| !analyze),
                if *analyze.get() { "Stop analyzing" } else { "Analyze" }
            }
            button {
                class: "button",
                style: "top: {WIDGET_HEIGHT}px",
                onclick: |_| {
                    log::info!("Quitting game..");
                    std::process::exit(0)
                },
                "Quit"
            }
        }
    })
}
