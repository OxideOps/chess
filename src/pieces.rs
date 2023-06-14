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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy)]
pub struct Direction {
    pub x: i8,
    pub y: i8,
}

impl Add<Direction> for Position {
    type Output = Self;

    fn add(self, dir: Direction) -> Self {
        Self {
            x: self.x.wrapping_add(dir.x as usize),
            y: self.y.wrapping_add(dir.y as usize),
        }
    }
}

impl AddAssign<Direction> for Position {
    fn add_assign(&mut self, other: Direction) {
        *self = Self {
            x: self.x.wrapping_add(other.x as usize),
            y: self.y.wrapping_add(other.y as usize),
        };
    }
}
