
use std::time::Duration;

use crate::chess_widget::*;
use chess::game::Game;
use chess::pieces::Color;
use chess::player::{Player, PlayerKind};

use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    GameContext::new(cx).render(cx)
}
