use crate::chess_widget::ChessWidgetProps;
use chess::game::Game;
use chess::moves::Move;

use dioxus::prelude::*;

pub fn create_game_socket(
    cx: Scope<ChessWidgetProps>,
    game: &UseRef<Game>,
) -> Option<Coroutine<Move>> {
    None
}
