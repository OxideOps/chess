use super::{RoundList, Timer};

use dioxus::prelude::*;
use std::time::Duration;

#[component]
pub(crate) fn InfoBar(cx: Scope, start_time: Duration) -> Element {
    cx.render(rsx! {
        div { class: "info-bar-container",
            Timer { start_time: *start_time }
            RoundList {}
        }
    })
}
