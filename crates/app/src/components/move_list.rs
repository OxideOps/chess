use chess_core::{Game, shakmaty::Position};
use dioxus::prelude::*;

/// One entry in the list: the ply it leads to and its SAN.
#[derive(Clone, PartialEq)]
struct Entry {
    ply: usize,
    san: String,
}

/// Numbered move pairs. Clicking a move shows the position after it.
#[component]
pub fn MoveList(game: Signal<Game>) -> Element {
    let g = game.read();
    let cursor = g.cursor();
    let start = g.start_position();
    let mut number = start.fullmoves().get();
    let mut rows: Vec<(u32, Option<Entry>, Option<Entry>)> = Vec::new();

    for (i, played) in g.moves().iter().enumerate() {
        let entry = Entry {
            ply: i + 1,
            san: played.san.to_string(),
        };
        let white_move = (i % 2 == 0) == start.turn().is_white();
        if white_move {
            rows.push((number, Some(entry), None));
        } else {
            match rows.last_mut() {
                Some((_, _, black)) if black.is_none() => *black = Some(entry),
                // A game that started with Black to move has no white move in its first row.
                _ => rows.push((number, None, Some(entry))),
            }
            number += 1;
        }
    }
    drop(g);

    rsx! {
        ol { class: "move-list",
            for (number, white, black) in rows {
                li {
                    span { class: "number", "{number}." }
                    MoveCell { game, cursor, entry: white }
                    MoveCell { game, cursor, entry: black }
                }
            }
        }
    }
}

#[component]
fn MoveCell(game: Signal<Game>, cursor: usize, entry: Option<Entry>) -> Element {
    let Some(entry) = entry else {
        return rsx! { span { class: "move empty", "…" } };
    };
    let ply = entry.ply;
    rsx! {
        button {
            r#type: "button",
            class: "move",
            class: if ply == cursor { "current" },
            onclick: move |_| game.write().go_to_ply(ply),
            "{entry.san}"
        }
    }
}
