#![feature(let_chains)]
#![feature(stmt_expr_attributes)]

#[cfg(not(feature = "ssr"))]
mod client;
mod common;
mod server;

#[cfg(not(feature = "ssr"))]
pub use client::launch;
pub use common::args;
#[cfg(feature = "ssr")]
pub use server::launch;
