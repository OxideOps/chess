use chess::Position;
use dioxus::prelude::*;

use super::super::{
    components::board::to_point,
    shared_states::{BoardSize, Perspective},
};

#[component]
pub(crate) fn BoardSquare(cx: Scope, class: String, position: Position, hovered: bool) -> Element {
    let board_size = **use_shared_state::<BoardSize>(cx)?.read();
    let perspective = **use_shared_state::<Perspective>(cx)?.read();
    let top_left = to_point(board_size, perspective, position);
    let border_class = if *hovered && class != "text-square" {
        "border-hover-square"
    } else {
        ""
    };
    cx.render(rsx! {
        div {
            class: "board-square {class} {border_class}",
            style: "
                left: {top_left.x}px;
                top: {top_left.y}px;
                width: {board_size / 8}px;
                height: {board_size / 8}px;
            ",
            "{position}"
        }
    })
}
