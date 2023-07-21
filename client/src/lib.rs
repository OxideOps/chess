#![allow(non_snake_case)]
pub mod app;
pub mod chess_widget;
#[cfg(not(target_arch = "wasm32"))]
pub mod desktop;
#[cfg(target_arch = "wasm32")]
pub mod web;
