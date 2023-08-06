use crate::round_list::RoundList;
use crate::timer::Timer;

use chess::game::Game;
use chess::game_status::GameStatus;
use dioxus::prelude::*;
use std::time::Duration;

#[derive(Props, PartialEq)]
pub struct InfoBarProps<'a> {
    game: &'a UseRef<Game>,
    time: Duration,
    left: u32,
}

pub fn InfoBar<'a>(cx: Scope<'a, InfoBarProps<'a>>) -> Element<'a> {
    let game_status = cx.props.game.with(|game| game.status);
    let classes = if game_status == GameStatus::Check {
        "mb-4 bg-red-600/75"
    } else {
        "mb-4"
    };
    cx.render(rsx! {
        div { class: "info-bar-container", style: "left: {cx.props.left}px;",
            Timer { game: cx.props.game, time: cx.props.time }
            p { class: "{classes}", "GameStatus: {game_status:?}" }
            RoundList { game: cx.props.game }
        }
    })
}
