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
#[cfg(target_arch = "wasm32")]
#[path = "stockfish_interface_web.rs"]
pub mod stockfish_interface;
#[cfg(not(target_arch = "wasm32"))]
#[path = "stockfish_interface_desktop.rs"]
pub mod stockfish_interface;
pub mod timer;
pub mod widget;


pub fn setup_config(config: &mut LaunchBuilder) {
    #[cfg(feature = "desktop")]
    crate::desktop::setup_config(config);

    #[cfg(feature = "web")]
    crate::web::setup_config(config);

    #[cfg(feature = "server")]
    crate::server::setup_config(config);
}