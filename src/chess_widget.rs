use dioxus::prelude::*;

#[inline_props]
pub fn ChessWidget(cx: Scope, size: u32) -> Element {
    render! {
        style { include_str!("../styles/chess_widget.css") }
        img {
            src: "images/board.png",
            class: "images",
            width: "{size}",
            height: "{size}",
        }
    }
}
