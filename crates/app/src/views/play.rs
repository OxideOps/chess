use chess_core::{Color, Game};
use dioxus::prelude::*;

use crate::components::{Board, Controls, MoveList, status_text};

/// A local two-player game: both sides are moved from this screen.
#[component]
pub fn Play() -> Element {
    let game = use_signal(Game::new);
    let orientation = use_signal(|| Color::White);

    let (status, turn, fen) = {
        let g = game.read();
        (g.status(), g.turn(), g.fen())
    };

    rsx! {
        div { class: "play",
            Board { game, orientation: orientation() }
            aside { class: "sidebar",
                p { class: "status", "{status_text(status, turn)}" }
                MoveList { game }
                Controls { game, orientation }
                label { class: "fen",
                    span { "FEN" }
                    input { readonly: true, value: "{fen}" }
                }
            }
        }
    }
}
