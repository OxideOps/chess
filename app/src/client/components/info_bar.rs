use dioxus::prelude::*;

use super::{RoundList, Timer};

#[component]
pub(crate) fn InfoBar(cx: Scope) -> Element {
    cx.render(rsx! {
        div { class: "info-bar-container",
            Timer {}
            RoundList {}
        }
    })
}
