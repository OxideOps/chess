use crate::moves::Move;
use std::ops::{Add, AddAssign};

#[derive(Clone, Copy, PartialEq)]
pub enum Piece {
    Pawn(Player),
    Knight(Player),
    Bishop(Player),
    Rook(Player),
    Queen(Player),
    King(Player),
}

impl Piece {
    pub fn get_player(&self) -> Player {
        match *self {
            Self::Pawn(player)
            | Self::Knight(player)
            | Self::Bishop(player)
            | Self::Rook(player)
            | Self::Queen(player)
            | Self::King(player) => player,
        }
    }
    pub fn can_snipe(&self) -> bool {
        match self {
            Self::Bishop(..) | Self::Rook(..) | Self::Queen(..) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Player {
    White,
    Black,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

impl Add<Move> for Position {
    type Output = Self;

    fn add(self, m: Move) -> Self {
        Self {
            x: self.x.wrapping_add(m.dx as usize),
            y: self.y.wrapping_add(m.dy as usize),
        }
    }
}

impl AddAssign<Move> for Position {
    fn add_assign(&mut self, other: Move) {
        *self = Self {
            x: self.x.wrapping_add(other.dx as usize),
            y: self.y.wrapping_add(other.dy as usize),
        };
    }
}

// Add unit tests at the bottom of each file. Each tests module should only have access to super (non integration)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_position() {
        let mut p = Position::new(0, 0);
        let m_up = Move::new(0, 1);
        let m_right = Move::new(1, 0);

        for _ in 0..10 {
            p = p + m_right + m_up
        }
        assert_eq!(p, Position::new(10, 10))
    }
}
