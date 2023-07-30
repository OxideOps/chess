use chess::game::Game;
use dioxus::prelude::*;

#[derive(Props, PartialEq)]
pub struct RoundListProps<'a> {
    game: &'a UseRef<Game>,
}

pub fn RoundList<'a>(cx: Scope<'a, RoundListProps<'a>>) -> Element<'a> {
    cx.render(rsx! {
        div {
            class: "rounds-container",
            style: "position: relative; overflow-y: auto;",
            p { "Rounds:" },
            cx.props.game.with(|game| {
                game.get_rounds_info().into_iter().enumerate().map(|(i, info)| {
                    let fill = info.2;
                    let style = if fill {
                        "margin-bottom: 15px; background-color: rgba(173, 216, 230, 0.5);"
                    }
                    else {
                        "margin-bottom: 15px;"
                    };

                    rsx! {
                        div {
                            style: "{style}",
                            tr {
                                td {
                                    style: "padding-right: 15px;",
                                    "{i + 1}."
                                }
                                td {
                                    style: "padding-right: 15px;",
                                    "{info.0}"
                                }
                                td {
                                    style: "padding-right: 15px;",
                                    "{info.1}"
                                }
                            }
                        }
                    }
                })
            })
        }
    })
}
