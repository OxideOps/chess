#![allow(non_snake_case)]
pub mod app;
pub mod arrow;
pub mod arrows;
pub mod board;
pub mod game_socket;
pub mod info_bar;
pub mod mouse_click;
pub mod round_list;
pub mod shared_states;
pub mod stockfish_client;
pub mod stockfish;
pub mod timer;
pub mod widget;

pub use stockfish::stockfish_interface;