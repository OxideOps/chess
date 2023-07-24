#[cfg(not(target_arch = "wasm32"))]
use crate::desktop::game_socket::create_game_socket;
#[cfg(target_arch = "wasm32")]
use crate::web::game_socket::create_game_socket;
use std::time::Duration;

use crate::chess_widget::*;
use chess::game::Game;
use chess::pieces::Color;
use chess::player::{Player, PlayerKind};

use dioxus::prelude::*;

pub fn App(cx: Scope) -> Element {
    //Choose Remote or Local here (Local default)
    let white_player = Player::with_color(Color::White);
    let black_player = Player::with_color(Color::Black);

    let game = use_ref(cx, || {
        Game::builder().duration(Duration::from_secs(3600)).build()
    });
    let write_socket = if has_remote_player(&white_player, &black_player) {
        create_game_socket(cx, game)
    } else {
        None
    };
    
    GameContext {
        game,
        mouse_down_state: use_state(cx, || None),
        dragging_point_state: use_state(cx, || None),
        write_socket,
        white_player,
        black_player,
    }
    .render(cx)
}

pub fn has_remote_player(white_player: &Player, black_player: &Player) -> bool {
    [white_player.kind, black_player.kind].contains(&PlayerKind::Remote)
}
