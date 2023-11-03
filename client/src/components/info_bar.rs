use std::time::Duration;

use dioxus::prelude::*;

use super::{RoundList, Timer};

#[component]
pub(crate) fn InfoBar(cx: Scope, start_time: Duration) -> Element {
    cx.render(rsx! {
        div { class: "info-bar-container",
            Timer { start_time: *start_time }
            RoundList {}
        }
    })
}
