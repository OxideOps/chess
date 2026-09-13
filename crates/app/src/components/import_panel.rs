use chess_core::Game;
use dioxus::prelude::*;

/// Load a position from FEN or a game from PGN, and copy the current game out.
#[component]
pub fn ImportPanel(game: Signal<Game>) -> Element {
    let mut fen_text = use_signal(String::new);
    let mut pgn_text = use_signal(String::new);
    let mut error: Signal<Option<String>> = use_signal(|| None);
    let current_pgn = game.read().pgn();

    let mut load = move |result: Result<Game, chess_core::GameError>| match result {
        Ok(loaded) => {
            game.set(loaded);
            error.set(None);
        }
        Err(err) => error.set(Some(err.to_string())),
    };

    rsx! {
        section { class: "import",
            form {
                onsubmit: move |event| {
                    event.prevent_default();
                    load(Game::from_fen(fen_text.read().trim()));
                },
                label { r#for: "import-fen", "FEN" }
                div { class: "row",
                    input {
                        id: "import-fen",
                        placeholder: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                        spellcheck: false,
                        value: "{fen_text}",
                        oninput: move |event| fen_text.set(event.value()),
                    }
                    button { r#type: "submit", "Load" }
                }
            }
            form {
                onsubmit: move |event| {
                    event.prevent_default();
                    load(Game::from_pgn(&pgn_text.read()));
                },
                label { r#for: "import-pgn", "PGN" }
                textarea {
                    id: "import-pgn",
                    rows: 4,
                    placeholder: "1. e4 e5 2. Nf3 …",
                    spellcheck: false,
                    value: "{pgn_text}",
                    oninput: move |event| pgn_text.set(event.value()),
                }
                div { class: "row",
                    button { r#type: "submit", "Load" }
                }
            }
            if let Some(message) = error() {
                p { class: "error", role: "alert", "{message}" }
            }
            label { r#for: "export-pgn", "Current game" }
            textarea { id: "export-pgn", rows: 3, readonly: true, value: "{current_pgn}" }
        }
    }
}
