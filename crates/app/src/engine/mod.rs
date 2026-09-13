//! Running a chess engine next to the UI.
//!
//! [`worker`] starts Stockfish and moves text in and out; [`analysis`] turns
//! that into a hook a view can use: give it a position, read back lines.

mod analysis;
mod worker;

pub use analysis::{Analysis, EngineStatus, use_analysis};
pub use worker::{Engine, EngineEvent};
