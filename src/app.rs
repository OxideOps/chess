use crate::chess_widget::ChessWidget;
use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    render! {
        ChessWidget {},
    }
}
