#![feature(let_chains)]
#![feature(stmt_expr_attributes)]
mod arrows;
#[cfg(not(feature = "server"))]
mod components;
mod game_socket;
mod helpers;
mod launcher;
mod mouse_click;
mod shared_states;
#[cfg(not(feature = "server"))]
mod stockfish;
mod system_info;

pub use launcher::launch;
