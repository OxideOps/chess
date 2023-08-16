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
    if black_player.read().kind == PlayerKind::Local
        && white_player.read().kind != PlayerKind::Local
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

    cx.render(rsx! {
        style { include_str!("../../styles/output.css") }
        Widget {
            game_id: *game_id.get(),
            white_player: white_player.to_owned(),
            black_player: black_player.to_owned(),
            perspective: *perspective.get(),
            start_time: Duration::from_secs(3600),
            height: WIDGET_HEIGHT
        }
        button {
            onclick: |_| {
                let white_player = white_player.to_owned();
                let black_player = black_player.to_owned();
                let perspective = perspective.to_owned();
                let game_id = game_id.to_owned();
                cx.spawn(async move {
                    match setup_remote_game().await {
                        Ok(info) => {
                            log::info!("Setting up remote game: {info:?}");
                            game_id.set(Some(info.game_id));
                            let player = match info.local_color {
                                Color::White => black_player.to_owned(),
                                Color::Black => white_player.to_owned(),
                            };
                            player.write().kind = PlayerKind::Remote;
                            perspective.set(get_default_perspective(&white_player, &black_player));
                        }
                        Err(err) => log::error!("Error starting remote game: {err:?}"),
                    }
                })
            },
            class: "buttons",
            style: "top: {WIDGET_HEIGHT}px",
            "Play Remote"
        }
        button {
            onclick: |_| {
                let perspective = perspective.to_owned();
                match perspective.get() {
                    Color::White => perspective.set(Color::Black),
                    Color::Black => perspective.set(Color::White),
                }
            },
            class: "buttons",
            style: "top: {WIDGET_HEIGHT}px",
            "Flip Board"
        }
    })
}
