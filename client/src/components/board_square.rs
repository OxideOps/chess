use dioxus::{html::geometry::ClientPoint, prelude::*};

#[component]
pub(crate) fn BoardSquare(
    cx: Scope,
    class: String,
    top_left: ClientPoint,
    board_size: u32,
    hovered: bool,
) -> Element {
    let border_class = if *hovered { "border-square" } else { "" };
    cx.render(rsx! {
        div {
            class: "{class} {border_class}",
            style: "
                left: {top_left.x}px;
                top: {top_left.y}px;
                width: {board_size / 8}px;
                height: {board_size / 8}px;
            "
        }
    })
}
