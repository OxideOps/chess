use crate::game::ChessError;
use std::mem;

#[derive(Clone, Copy, PartialEq)]
pub enum Piece {
    Pawn(Player),
    Knight(Player),
    Bishop(Player),
    Rook(Player),
    Queen(Player),
    King(Player),
    None,
}

impl Piece {
    pub fn is_none(&self) -> bool {
        *self == Self::None
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn take(&mut self) -> Self {
        mem::replace(self, Self::None)
    }

    pub fn borrow_player(&self) -> Result<&Player, ChessError> {
        match self {
            &Self::Pawn(ref player) => Ok(player),
            &Self::Knight(ref player) => Ok(player),
            &Self::Bishop(ref player) => Ok(player),
            &Self::Rook(ref player) => Ok(player),
            &Self::Queen(ref player) => Ok(player),
            &Self::King(ref player) => Ok(player),
            &Self::None => Err(ChessError::NoPieceAtPosition),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Player {
    White,
    Black,
}

#[derive(Clone, Copy)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}
