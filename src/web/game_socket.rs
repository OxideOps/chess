use crate::chess_widget::ChessWidgetProps;
use crate::game::Game;
use crate::moves::Move;

use dioxus::prelude::*;
use once_cell::sync::Lazy;
use std::sync::RwLock;

pub fn create_game_socket<'a>(
    _cx: &'a Scoped<'a, ChessWidgetProps>,
    _board_state_hash: &UseState<u64>,
    _game: &'static Lazy<RwLock<Game>>,
) -> Option<&'a Coroutine<Move>> {
    None
}
