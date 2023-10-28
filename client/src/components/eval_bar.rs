use crate::helpers::sigmoid;
use crate::shared_states::Eval;
use chess::color::Color;
use dioxus::prelude::*;

const EVAL_SENSITIVITY: f64 = 1.0 / 800.0;
#[component]
pub(crate) fn EvalBar(cx: Scope, perspective: Color) -> Element {
    let eval = **use_shared_state::<Eval>(cx).unwrap().read();
    let percent = 100.0 * sigmoid(EVAL_SENSITIVITY * eval);
    let direction = match perspective {
        Color::White => "top",
        Color::Black => "bottom",
    };
    cx.render(rsx! {
        div {
            class: "eval-container",
            style: "background: linear-gradient(to {direction}, white 0%, white {percent}%, black {percent}%, black 100%);"
        }
    })
}
