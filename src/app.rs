use crate::chess_widget::ChessWidget;
use dioxus::prelude::*;
use dioxus_desktop::{use_window, LogicalSize};

const WINDOW_SIZE: u32 = 800;

pub fn App(cx: Scope) -> Element {
    use_window(cx).set_inner_size(LogicalSize {
        width: WINDOW_SIZE,
        height: WINDOW_SIZE,
    });
    use_window(cx).set_title("Chess");
    render! {
        ChessWidget {},
    }
}
