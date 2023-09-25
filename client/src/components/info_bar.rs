use crate::components::RoundList;
use crate::components::Timer;

use dioxus::prelude::*;
use std::time::Duration;

#[derive(Props, PartialEq)]
pub(crate) struct InfoBarProps {
    start_time: Duration,
    left: u32,
}

pub(crate) fn InfoBar(cx: Scope<InfoBarProps>) -> Element {
    cx.render(rsx! {
        div { class: "info-bar-container", style: "left: {cx.props.left}px;",
            Timer { start_time: cx.props.start_time },
            RoundList {}
        }
    })
}
