use chess_core::{Color, Game};
use dioxus::prelude::*;

use crate::{
    components::{Board, Controls, EnginePanel, EvalBar, ImportPanel, MoveList, status_text},
    engine::use_analysis,
};

const ANALYSIS_CSS: Asset = asset!("/assets/analysis.css");

/// Free analysis: set up any position, step through a game, and let the
/// engine comment. Moves can be made from anywhere in the history.
#[component]
pub fn Analysis() -> Element {
    let game = use_signal(Game::new);
    let orientation = use_signal(|| Color::White);
    let engine_on = use_signal(|| true);

    // The engine follows the viewed position, unless it's off or the game is over there.
    let target = use_memo(move || {
        let g = game.read();
        (engine_on() && !g.status().is_game_over()).then(|| g.fen())
    });
    let analysis = use_analysis(target, 3);

    let (status, turn, fen) = {
        let g = game.read();
        (g.status(), g.turn(), g.fen())
    };
    let (score, arrows) = {
        let a = analysis.read();
        // Only decorate the board with lines that are about the position it shows.
        let current = a.fen.as_deref() == Some(fen.as_str());
        let best = a.best().filter(|_| current);
        let score = best.map(|l| l.score.for_white(a.turn));
        let arrows: Vec<(chess_core::Square, chess_core::Square)> = best
            .and_then(|l| l.best_move())
            .and_then(|m| match m {
                chess_core::shakmaty::uci::UciMove::Normal { from, to, .. } => Some((from, to)),
                _ => None,
            })
            .into_iter()
            .collect();
        (score, arrows)
    };

    rsx! {
        document::Stylesheet { href: ANALYSIS_CSS }
        div { class: "analysis",
            div { class: "board-with-bar",
                EvalBar { score, orientation: orientation() }
                Board { game, orientation: orientation(), analysis: true, arrows }
            }
            aside { class: "sidebar",
                EnginePanel { game, analysis, enabled: engine_on }
                p { class: "status", "{status_text(status, turn)}" }
                MoveList { game }
                Controls { game, orientation }
                ImportPanel { game }
            }
        }
    }
}
