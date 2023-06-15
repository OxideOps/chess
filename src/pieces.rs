use std::mem;
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

impl Add for Position {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl AddAssign for Position {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
        };
    }
}

// Add unit tests at the bottom of each file. Each tests module should only have access to super (non integration)
#[cfg(test)]
mod tests {
    use super::Position;

    #[test]
    fn test_add_position() {
        let p = Position { x: 1, y: 1 };
        assert_eq!(p + p, Position { x: 2, y: 2 });

        let mut q = p;
        q += p;
        assert_eq!(q, p + p)
    }
}
