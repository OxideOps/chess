use chess::{Color, Game};
use dioxus::prelude::*;

use super::{
    super::{
        shared_states::{Analyze, BoardSize, GameId, Perspective},
        stockfish::Eval,
    },
    Widget,
};

const WIDGET_HEIGHT: u32 = 800;

pub(crate) fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, || Eval::Centipawns(0));
    use_shared_state_provider(cx, || GameId(None));
    use_shared_state_provider(cx, Game::new);
    use_shared_state_provider(cx, || BoardSize(WIDGET_HEIGHT));
    use_shared_state_provider(cx, || Perspective(Color::White));
    use_shared_state_provider(cx, || Analyze(false));

    cx.render(rsx! {
        style { include_str!("../../../styles/output.css") }
        Widget {}
    })
}
