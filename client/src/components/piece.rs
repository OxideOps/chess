use dioxus::{html::geometry::ClientPoint, prelude::*};
use dioxus_signals::Signal;

#[component]
pub fn Piece(
    cx: Scope,
    image: String,
    top_left_starting: ClientPoint,
    size: u32,
    is_dragging: bool,
    dragging_point: Signal<ClientPoint>,
) -> Element {
    let top_left = if *is_dragging {
        *dragging_point.read()
    } else {
        *top_left_starting
    };
    let z_index = cx.props.is_dragging as u32 + 1; // 🏌️

    cx.render(rsx! {
        img {
            src: "{image}",
            class: "images",
            style: "left: {top_left.x}px; top: {top_left.y}px; z-index: {z_index}",
            width: "{size}",
            height: "{size}"
        }
    })
}
