#![allow(non_snake_case)]
pub mod app;
pub mod arrow;
pub mod arrows;
pub mod board;
pub mod game_socket;
pub mod info_bar;
pub mod mouse_click;
pub mod round_list;
#[cfg(not(target_arch = "wasm32"))]
pub mod stockfish_client;
pub mod timer;
pub mod widget;
