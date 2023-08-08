use chess::game::Game;
use dioxus::prelude::*;

#[derive(Props, PartialEq)]
pub struct RoundListProps<'a> {
    game: &'a UseRef<Game>,
}

pub fn RoundList<'a>(cx: Scope<'a, RoundListProps<'a>>) -> Element<'a> {
    cx.render(rsx! {
        div { class: "rounds-container",
            p { "Rounds:" }
            table { class: "place-content-center",
                cx.props.game.with(|game| {
                    let current_round = game.get_current_round();
                    game.get_rounds_info().into_iter().enumerate().map(move |(i, info)| {
                        let classes = if i + 1 == current_round {
                            "mb-4 bg-gmv-600/75"
                        } else {
                            "mb-4"
                        };

                        rsx! {
                            tr {
                                class: "{classes}",
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
                    })
                })
            }
        }
    })
}
