#![feature(let_chains)]
#![feature(stmt_expr_attributes)]
mod arrows;
pub mod components;
mod game_socket;
mod helpers;
mod launcher;
mod mouse_click;
mod shared_states;
mod stockfish;
mod system_info;

pub use launcher::launch;
