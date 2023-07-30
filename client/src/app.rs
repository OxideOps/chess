use crate::widget::Widget;
use std::time::Duration;

use chess::{
    color::Color,
    player::{Player, PlayerKind},
};
use dioxus::prelude::*;
use server_functions::setup_remote_game::setup_remote_game;

const WIDGET_HEIGHT: u32 = 800;

pub fn App(cx: Scope) -> Element {
    let white_player = use_ref(cx, || Player::with_color(Color::White));
    let black_player = use_ref(cx, || Player::with_color(Color::Black));
    let game_id = use_state::<Option<u32>>(cx, || None);

    cx.render(rsx! {
        Widget {
            white_player: white_player.to_owned(),
            black_player: black_player.to_owned(),
            time: Duration::from_secs(3600),
            height: WIDGET_HEIGHT,
        },
        button {
            onclick: |_| {
                let white_player = white_player.to_owned();
                let black_player = black_player.to_owned();
                let game_id = game_id.to_owned();
                cx.spawn(async move {
                    match setup_remote_game().await {
                        Ok((new_game_id, color)) => {
                            log::info!("Got game_id: {new_game_id:?} and color: {color:?}");
                            game_id.set(Some(new_game_id));
                            let player = match color {
                                Color::White => black_player.to_owned(),
                                Color::Black => white_player.to_owned(),
                            };
                            player.with_mut(|player| player.kind = PlayerKind::Remote);
                        },
                        Err(err) => log::error!("Error starting remote game: {err:?}"),
                    }
                })
            },
            style: "position: absolute; top: {WIDGET_HEIGHT}px",
            "Play Remote",
        }
    })
}
