use crate::chess_widget::GameContext;

use dioxus::prelude::*;

pub fn App<'cx>(cx: &'cx Scoped) -> Element<'cx> {
    GameContext::new(cx).render()
}
