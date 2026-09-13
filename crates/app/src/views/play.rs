use chess_core::{Color, Game, GameStatus};
use dioxus::prelude::*;

use crate::components::{Board, Controls, MoveList, side_name};

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

fn status_text(status: GameStatus, turn: Color) -> String {
    match status {
        GameStatus::Ongoing => format!("{} to move", side_name(turn)),
        GameStatus::Check => format!("{} to move — check", side_name(turn)),
        GameStatus::Checkmate { winner } => format!("Checkmate — {} wins", side_name(winner)),
        GameStatus::Stalemate => "Draw by stalemate".into(),
        GameStatus::InsufficientMaterial => "Draw by insufficient material".into(),
        GameStatus::FiftyMoveRule => "Draw by the fifty-move rule".into(),
        GameStatus::ThreefoldRepetition => "Draw by threefold repetition".into(),
    }
}
