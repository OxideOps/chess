use crate::round_list::RoundList;
use crate::timer::Timer;

use chess::game::Game;
use chess::game_status::GameStatus;
use dioxus::prelude::*;
use std::time::Duration;

#[derive(Props, PartialEq)]
pub struct InfoBarProps {
    start_time: Duration,
    left: u32,
}

pub fn InfoBar(cx: Scope<InfoBarProps>) -> Element {
    let game_status = use_shared_state::<Game>(cx).unwrap().read().status;
    let classes = if matches!(game_status, GameStatus::Check(..)) {
        "mb-4 bg-red-600/75"
    } else {
        "mb-4"
    };
    cx.render(rsx! {
        div { class: "info-bar-container", style: "left: {cx.props.left}px;",
            Timer { start_time: cx.props.start_time }
            p { class: "{classes}", "GameStatus: {game_status:?}" }
            RoundList { }
        }
    })
}
