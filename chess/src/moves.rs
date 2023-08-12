use crate::piece::Piece;
use crate::position::Position;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Move {
    pub from: Position,
    pub to: Position,
}

impl Move {
    pub fn new(from: Position, to: Position) -> Self {
        Self { from, to }
    }

    pub fn inverse(&self) -> Self {
        Self {
            from: self.to,
            to: self.from,
        }
    }

    // creates move from the "long algebraic notation" that stockfish uses
    pub fn from_lan(lan: &str) -> Option<Self> {
        Some(Self::new(
            Position::new(lan.chars().nth(0)? as usize, lan.chars().nth(1)? as usize),
            Position::new(lan.chars().nth(2)? as usize, lan.chars().nth(3)? as usize),
        ))
    }

    pub fn to_str(&self, piece: Piece) -> String {
        format!("{}{}", piece, self.to)
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -> {}", self.from, self.to)
    }
}
