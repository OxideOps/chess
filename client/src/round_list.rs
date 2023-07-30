use chess::game::Game;
use dioxus::prelude::*;

#[derive(Props, PartialEq)]
pub struct RoundListProps<'a> {
    game: &'a UseRef<Game>,
}

pub fn RoundList<'a>(cx: Scope<'a, RoundListProps<'a>>) -> Element<'a>{
    cx.render(rsx! {
        p { "Rounds:" },
        cx.props.game.with(|game| {
            game.get_rounds_str().into_iter().enumerate().map(|(i, moves)| {
                rsx! {
                    div {
                        style: "margin-bottom: 15px;",
                        tr {
                            td {
                                style: "padding-right: 15px;" ,
                                "{i + 1}."
                            }
                            td {
                                style: "padding-right: 15px;",
                                "{moves.0}"
                            }
                            td {
                                style: "padding-right: 15px;",
                                "{moves.1}"
                            }
                        }
                    }
                }
            })
        })
    })

}