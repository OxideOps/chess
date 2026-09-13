use chess_core::{Color, engine::Score};
use dioxus::prelude::*;

/// A vertical bar showing who is better. `score` is from White's point of
/// view; `None` shows an even bar with no label.
#[component]
pub fn EvalBar(score: Option<Score>, orientation: Color) -> Element {
    // Never let either side vanish completely, so the bar still reads as a bar.
    let white = score.map_or(0.5, |s| s.bar_fraction().clamp(0.04, 0.96));
    let label = score.map(|s| match s {
        Score::Cp(cp) => format!("{:.1}", f64::from(cp.abs()) / 100.0),
        Score::Mate(n) => format!("#{}", n.abs()),
    });
    let white_ahead = white >= 0.5;

    rsx! {
        div {
            class: "eval-bar",
            class: if orientation.is_black() { "flipped" },
            title: match score {
                Some(s) => format!("Evaluation {s} (White's point of view)"),
                None => "No evaluation".to_string(),
            },
            div { class: "white", style: "height: {white * 100.0}%" }
            if let Some(label) = label {
                span {
                    class: "label",
                    class: if white_ahead { "for-white" } else { "for-black" },
                    "{label}"
                }
            }
        }
    }
}
