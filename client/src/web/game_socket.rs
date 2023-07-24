use crate::chess_widget::ChessWidgetProps;
use chess::game::Game;
use chess::moves::Move;

use dioxus::prelude::*;

pub fn create_game_socket<'cx>(
    cx: Scope<'cx, ChessWidgetProps>,
    game: &UseRef<Game>,
) -> Option<&'cx Coroutine<Move>> {
    None
}
