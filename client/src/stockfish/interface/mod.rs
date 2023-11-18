#[cfg(feature = "desktop")]
mod desktop_interface;
#[cfg(target_arch = "wasm32")]
mod web_interface;

#[cfg(feature = "desktop")]
pub(crate) use desktop_interface::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use web_interface::*;
