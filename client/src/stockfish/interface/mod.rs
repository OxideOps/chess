#[cfg(not(target_arch = "wasm32"))]
mod desktop_interface;
#[cfg(target_arch = "wasm32")]
mod web_interface;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop_interface::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use web_interface::*;
