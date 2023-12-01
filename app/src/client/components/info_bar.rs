use dioxus::prelude::*;

use super::{RoundList, Timer};

#[component]
pub(crate) fn InfoBar(cx: Scope, is_local_game: bool) -> Element {
    cx.render(rsx! {
        div { class: "info-bar-container",
            if !is_local_game {
                rsx! { Timer {} }
            }
            RoundList {}
        }
    })
}
