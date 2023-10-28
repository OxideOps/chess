use crate::helpers::sigmoid;
use crate::shared_states::Eval;
use chess::color::Color;
use dioxus::prelude::*;

const EVAL_SENSITIVITY: f64 = 1.0 / 800.0;
#[component]
pub(crate) fn EvalBar(cx: Scope, perspective: Color) -> Element {
    let eval = **use_shared_state::<Eval>(cx).unwrap().read();
    let percent = 100.0 * sigmoid(EVAL_SENSITIVITY * eval);
    let (text_color, direction) = match perspective {
        Color::White => ("black", "top"),
        Color::Black => ("white", "bottom"),
    };
    cx.render(rsx! {
        div {
            class: "eval-container",
            style: "
                background: linear-gradient(
                    to {direction}, white 0%, white {percent}%, black {percent}%, black 100%
                );
                color: {text_color};
            ",
            "{eval / 100.0:.1}"
        }
    })
}
