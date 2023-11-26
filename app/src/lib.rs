#![feature(let_chains)]
#![feature(stmt_expr_attributes)]
#![feature(future_join)]

#[cfg(not(feature = "ssr"))]
mod client;
mod server;

#[cfg(not(feature = "ssr"))]
pub use client::launch;
#[cfg(feature = "ssr")]
pub use server::launch;
