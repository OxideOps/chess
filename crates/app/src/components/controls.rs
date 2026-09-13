use chess_core::{Color, Game};
use dioxus::prelude::*;

/// History navigation plus board-level actions.
#[component]
pub fn Controls(game: Signal<Game>, orientation: Signal<Color>) -> Element {
    let at_start = game.read().cursor() == 0;
    let at_end = !game.read().is_viewing_history();

    rsx! {
        div { class: "controls",
            button { r#type: "button", title: "First move", disabled: at_start,
                onclick: move |_| game.write().go_to_start(), "⏮" }
            button { r#type: "button", title: "Previous move", disabled: at_start,
                onclick: move |_| game.write().go_back(), "◀" }
            button { r#type: "button", title: "Next move", disabled: at_end,
                onclick: move |_| game.write().go_forward(), "▶" }
            button { r#type: "button", title: "Last move", disabled: at_end,
                onclick: move |_| game.write().go_to_end(), "⏭" }
        }
        div { class: "controls",
            button { r#type: "button",
                onclick: move |_| orientation.set(!orientation()), "Flip board" }
            button { r#type: "button",
                onclick: move |_| game.set(Game::new()), "New game" }
        }
    }
}
