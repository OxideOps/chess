//! Shared chess logic for the whole app.
//!
//! The rules themselves come from [`shakmaty`]; this crate adds a [`Game`]
//! (a position plus its move history, with navigation) and the wire
//! [`protocol`] used between client and server. Everything here compiles for
//! both native and `wasm32`, and nothing in it knows about the UI or the
//! database.

pub mod game;
pub mod protocol;

pub use game::{Game, GameError, GameStatus, PlayedMove};
pub use shakmaty;
pub use shakmaty::{Color, File, Move, Piece, Rank, Role, Square};
