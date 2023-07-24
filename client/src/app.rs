use crate::chess_widget::GameContext;

use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    GameContext::new(cx).render()
}
