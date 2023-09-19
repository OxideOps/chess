#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
pub mod desktop;

#[cfg(target_arch = "wasm32")]
pub use web::stockfish_interface_web as stockfish_interface;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::stockfish_interface_desktop as stockfish_interface;
