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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy)]
pub struct Move {
    pub dx: i8,
    pub dy: i8,
}

impl Move {
    pub const PAWN_MOVES: &'static [Move] = &[Move { dx: 0, dy: 1 }];
    
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
