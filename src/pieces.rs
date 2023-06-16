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
    pub fn get_player(self) -> Player {
        match self {
            Self::Pawn(player)
            | Self::Knight(player)
            | Self::Bishop(player)
            | Self::Rook(player)
            | Self::Queen(player)
            | Self::King(player) => player,
        }
    }
    pub fn get_moves(self) -> &'static [Move] {
        // not exactly sure how to handle pawns yet
        match self {
            Self::Rook(..) => Move::get_rook_moves(),
            Self::Bishop(..) => Move::get_bishop_moves(),
            Self::Knight(..) => Move::get_knight_moves(),
            Self::Queen(..) => Move::get_queen_moves(),
            Self::King(..) => Move::get_king_moves(),
            _ => Default::default(),
        }
    }
    pub fn can_snipe(self) -> bool {
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
    fn add_assign(&mut self, m: Move) {
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
        let m_up = Move { dx: 0, dy: 1 };
        let m_right = Move { dx: 1, dy: 0 };

        for _ in 0..10 {
            p = p + m_right + m_up
        }
        assert_eq!(p, Position { x: 10, y: 10 })
    }
}
