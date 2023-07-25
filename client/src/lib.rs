#![allow(non_snake_case)]
pub mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod desktop;
#[cfg(target_arch = "wasm32")]
pub mod web;
pub mod widget;
