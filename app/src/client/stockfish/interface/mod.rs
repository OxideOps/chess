#[cfg(feature = "desktop")]
mod desktop_interface;
#[cfg(feature = "web")]
mod web_interface;

#[cfg(feature = "desktop")]
pub(crate) use desktop_interface::*;
#[cfg(feature = "web")]
pub(crate) use web_interface::*;
