use chess_core::{
    Game,
    engine::{Score, pv_movetext},
};
use dioxus::prelude::*;

use crate::engine::{Analysis, EngineStatus};

/// What one engine line looks like on screen.
#[derive(Clone, PartialEq)]
struct LineView {
    score: Score,
    movetext: String,
    first_move: String,
}

/// Engine on/off switch, progress, and its best lines. Clicking a line
/// plays its first move on the board.
#[component]
pub fn EnginePanel(
    game: Signal<Game>,
    analysis: Signal<Analysis>,
    enabled: Signal<bool>,
) -> Element {
    let a = analysis.read();
    let status = a.status.clone();
    let name = a.name.clone().unwrap_or_else(|| "Stockfish".to_string());
    let depth = a.depth();
    let idle = a.fen.is_none();
    let searching = a.searching;
    let turn = a.turn;
    let lines: Vec<LineView> = match a.fen.as_deref().and_then(|fen| Game::from_fen(fen).ok()) {
        Some(from) => a
            .lines
            .iter()
            .map(|line| LineView {
                score: line.score.for_white(turn),
                movetext: pv_movetext(from.position(), &line.pv),
                first_move: line.best_move().map(|m| m.to_string()).unwrap_or_default(),
            })
            .collect(),
        None => Vec::new(),
    };
    drop(a);

    let summary = match (&status, enabled(), depth) {
        (EngineStatus::Failed(err), _, _) => format!("Engine unavailable: {err}"),
        (EngineStatus::Loading, _, _) => "Loading engine…".to_string(),
        (EngineStatus::Ready, false, _) => "Engine off".to_string(),
        // Nothing to analyse: the game is over at the viewed position.
        (EngineStatus::Ready, true, _) if idle => "Idle".to_string(),
        (EngineStatus::Ready, true, Some(d)) if searching => format!("Depth {d}…"),
        (EngineStatus::Ready, true, Some(d)) => format!("Depth {d}"),
        (EngineStatus::Ready, true, None) => "Thinking…".to_string(),
    };

    rsx! {
        section { class: "engine",
            header {
                label {
                    input {
                        r#type: "checkbox",
                        checked: enabled(),
                        disabled: matches!(status, EngineStatus::Failed(_)),
                        onchange: move |event| enabled.set(event.checked()),
                    }
                    span { class: "name", "{name}" }
                }
                span { class: "summary", "{summary}" }
            }
            if enabled() && !lines.is_empty() {
                ol { class: "lines",
                    for line in lines {
                        li {
                            button {
                                r#type: "button",
                                title: "Play {line.first_move}",
                                onclick: {
                                    let uci = line.first_move.clone();
                                    move |_| {
                                        if let Err(err) = game.write().play_here_uci(&uci).map(|_| ()) {
                                            dioxus::logger::tracing::warn!("engine line: {err}");
                                        }
                                    }
                                },
                                span { class: "score", "{line.score}" }
                                span { class: "moves", "{line.movetext}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
