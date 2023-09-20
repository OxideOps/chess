#[cfg(not(target_arch = "wasm32"))]
pub mod desktop_interface;
#[cfg(target_arch = "wasm32")]
pub mod web_interface;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop_interface::*;
#[cfg(target_arch = "wasm32")]
pub use web_interface::*;
