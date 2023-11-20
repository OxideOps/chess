use chess::Game;
use dioxus::prelude::*;

#[component]
pub(crate) fn RoundList(cx: Scope) -> Element {
    let game = use_shared_state::<Game>(cx)?.read();
    let current_round = game.get_current_round();

    cx.render(rsx! {
        p { "Rounds:" }
        div { class: "rounds-container",
            table { class: "place-content-center",
                for (i, info) in game.get_rounds_info().into_iter().enumerate() {
                    tr {
                        class: if i + 1 == current_round {
                            "mb-4 bg-gray-600/75"
                        } else {
                            "mb-4"
                        },
                        td {
                            "{i + 1}."
                        }
                        td {
                            "{info.white_string}"
                        }
                        td {
                            "{info.black_string}"
                        }
                    }
                }
            }
        }
    })
}
