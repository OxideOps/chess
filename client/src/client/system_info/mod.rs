#[cfg(feature = "desktop")]
pub(crate) use desktop::*;
#[cfg(feature = "web")]
pub(crate) use web::*;

#[cfg(feature = "desktop")]
mod desktop;
#[cfg(feature = "web")]
mod web;
