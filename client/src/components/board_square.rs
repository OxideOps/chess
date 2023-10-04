use chess::color::Color;
use chess::position::Position;
use dioxus::prelude::*;

use super::board::to_point;

#[component]
pub(crate) fn BoardSquare(
    cx: Scope,
    class: String,
    pos: Position,
    board_size: u32,
    perspective: Color,
) -> Element {
    let top_left = to_point(pos, *board_size, *perspective);
    cx.render(rsx! {
        div {
            class: "{class}",
            style: "
                left: {top_left.x}px;
                top: {top_left.y}px;
                width: {cx.props.board_size / 8}px;
                height: {cx.props.board_size / 8}px;
            "
        }
    })
}
