use crate::displacement::Displacement;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Not, Sub};

#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub enum Piece {
    Pawn(Color),
    Knight(Color),
    Bishop(Color),
    Rook(Color),
    Queen(Color),
    King(Color),
}

impl Piece {
    pub fn is_pawn(self) -> bool {
        matches!(self, Piece::Pawn(..))
    }

    pub fn get_player(self) -> Color {
        match self {
            Self::Pawn(player)
            | Self::Knight(player)
            | Self::Bishop(player)
            | Self::Rook(player)
            | Self::Queen(player)
            | Self::King(player) => player,
        }
    }

    pub fn get_vectors(self) -> &'static [Displacement] {
        match self {
            Self::Pawn(..) => panic!("Try calling `Displacement::get_pawn_*_vector()` instead"),
            Self::Rook(..) => Displacement::get_rook_vectors(),
            Self::Bishop(..) => Displacement::get_bishop_vectors(),
            Self::Knight(..) => Displacement::get_knight_vectors(),
            Self::Queen(..) => Displacement::get_queen_vectors(),
            Self::King(..) => Displacement::get_king_vectors(),
        }
    }

    pub fn can_snipe(self) -> bool {
        matches!(self, Self::Bishop(..) | Self::Rook(..) | Self::Queen(..))
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let piece = match self {
            Piece::Pawn(Color::White) => "♟",
            Piece::Knight(Color::White) => "♞",
            Piece::Bishop(Color::White) => "♝",
            Piece::Rook(Color::White) => "♜",
            Piece::Queen(Color::White) => "♛",
            Piece::King(Color::White) => "♚",
            Piece::Pawn(Color::Black) => "♙",
            Piece::Knight(Color::Black) => "♘",
            Piece::Bishop(Color::Black) => "♗",
            Piece::Rook(Color::Black) => "♖",
            Piece::Queen(Color::Black) => "♕",
            Piece::King(Color::Black) => "♔",
        };
        write!(f, "{}", piece)
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default, Hash)]
pub enum Color {
    #[default]
    White,
    Black,
}

impl Not for Color {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

impl Add<Displacement> for Position {
    type Output = Self;

    fn add(self, m: Displacement) -> Self::Output {
        Self {
            x: self.x.wrapping_add(m.dx as usize),
            y: self.y.wrapping_add(m.dy as usize),
        }
    }
}

impl Sub<Displacement> for Position {
    type Output = Self;

    fn sub(self, m: Displacement) -> Self::Output {
        Self {
            x: self.x.wrapping_sub(m.dx as usize),
            y: self.y.wrapping_sub(m.dy as usize),
        }
    }
}

impl AddAssign<Displacement> for Position {
    fn add_assign(&mut self, m: Displacement) {
        *self = Self {
            x: self.x.wrapping_add(m.dx as usize),
            y: self.y.wrapping_add(m.dy as usize),
        };
    }
}

// Add unit tests at the bottom of each file. Each tests module should only have access to super (non integration)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_position() {
        let mut p = Position { x: 0, y: 0 };
        let m_up = Displacement { dx: 0, dy: 1 };
        let m_right = Displacement { dx: 1, dy: 0 };

        for _ in 0..10 {
            p = p + m_right + m_up
        }
        assert_eq!(p, Position { x: 10, y: 10 })
    }
}
