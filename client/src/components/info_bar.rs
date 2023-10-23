use crate::components::RoundList;
use crate::components::Timer;

use dioxus::prelude::*;
use std::time::Duration;

#[component]
pub(crate) fn InfoBar(cx: Scope, start_time: Duration, left: u32) -> Element {
    cx.render(rsx! {
        div { class: "info-bar-container", style: "left: {left}px;",
            Timer { start_time: *start_time }
            RoundList {}
        }
    })
}
