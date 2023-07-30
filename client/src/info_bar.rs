use crate::timer::Timer;
use crate::round_list::RoundList;


use chess::game::Game;
use dioxus::prelude::*;
use std::time::Duration;



#[derive(Props, PartialEq)]
pub struct InfoBarProps<'a> {
    game: &'a UseRef<Game>,
    time: Duration,
    left: u32,
}

pub fn InfoBar<'a>(cx: Scope<'a, InfoBarProps<'a>>) -> Element<'a> {    
    cx.render(rsx! {
        div {
            class: "time-container",
            style: "position: absolute; left: {cx.props.left}px; top: 0px",
            Timer {
                game: cx.props.game,
                time: cx.props.time,
            },
            div {
                class: "moves-container",
                style: "position: relative; overflow-y: auto;",
                RoundList {
                    game: cx.props.game
                }
            }
        },
    })
}
