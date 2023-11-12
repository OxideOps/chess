use chess::Color;
use dioxus::prelude::*;

use crate::{helpers::sigmoid, shared_states::Perspective, stockfish::Eval};

const EVAL_SENSITIVITY: f64 = 1.0 / 800.0;

#[component]
pub(crate) fn EvalBar(cx: Scope) -> Element {
    let eval = *use_shared_state::<Eval>(cx)?.read();
    let perspective = **use_shared_state::<Perspective>(cx)?.read();
    let winning_player = eval.get_winning_player();
    let percent = match eval {
        Eval::Centipawns(cp) => 100.0 * sigmoid(EVAL_SENSITIVITY * cp as f64),
        Eval::Mate(mate) => 100.0 * (mate > 0) as u64 as f64,
    };
    let direction = match perspective {
        Color::White => "top",
        Color::Black => "bottom",
    };
    let justify = if perspective == winning_player {
        "end"
    } else {
        "start"
    };
    let text_color = match winning_player {
        Color::White => "black",
        Color::Black => "white",
    };
    cx.render(rsx! {
        div {
            class: "eval-container",
            style: "
                background: linear-gradient(
                    to {direction}, white 0%, white {percent}%, black {percent}%, black 100%
                );
                color: {text_color};
                justify-content: {justify};
            ",
            "{eval}"
        }
    })
}
