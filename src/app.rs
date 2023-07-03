use crate::chess_widget::ChessWidget;
use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use dioxus_desktop::{use_window, LogicalSize};

        const WINDOW_SIZE: u32 = 800;

        use_window(cx).set_inner_size(LogicalSize {
            width: WINDOW_SIZE,
            height: WINDOW_SIZE,
        });
        use_window(cx).set_title("Chess");
        use_window(cx).set_focus();
    }
    render! {
        ChessWidget {},
    }
}
