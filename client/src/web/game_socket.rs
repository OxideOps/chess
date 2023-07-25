use crate::widget::WidgetProps;
use chess::game::Game;
use chess::moves::Move;

use dioxus::prelude::*;

pub fn create_game_socket<'a>(
    _cx: Scope<'a, WidgetProps>,
    _game: &UseRef<Game>,
) -> Option<&'a Coroutine<Move>> {
    None
}
