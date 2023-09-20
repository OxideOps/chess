pub mod client;
pub mod stockfish_interface;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

#[cfg(target_arch = "wasm32")]
pub use web::interface;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::interface;
