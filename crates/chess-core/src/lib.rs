//! Shared chess logic for the whole app.
//!
//! The rules themselves come from [`shakmaty`]; this crate adds a [`Game`]
//! (a position plus its move history, with navigation) and the wire
//! [`protocol`] used between client and server, plus a [`pgn`] reader and
//! the [`engine`] text protocol used to talk to UCI engines. Everything here compiles for
//! both native and `wasm32`, and nothing in it knows about the UI or the
//! database.

pub mod engine;
pub mod game;
pub mod pgn;
pub mod protocol;
pub mod puzzle;

pub use game::{Game, GameError, GameStatus, PlayedMove};
pub use shakmaty;
pub use shakmaty::{Color, File, Move, Piece, Rank, Role, Square};
