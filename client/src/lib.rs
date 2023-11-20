#![feature(let_chains)]
#![feature(stmt_expr_attributes)]
#[cfg(not(feature = "server"))]
mod client;
#[cfg(feature = "server")]
mod server;

#[cfg(not(feature = "server"))]
pub use client::launch;
#[cfg(feature = "server")]
pub use server::launch;
