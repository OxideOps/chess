#[cfg(feature = "desktop")]
pub(crate) use desktop::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::*;

#[cfg(feature = "desktop")]
mod desktop;
#[cfg(target_arch = "wasm32")]
mod web;
