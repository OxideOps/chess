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
        *self == Piece::None
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn take(&mut self) -> Piece {
        mem::replace(self, Piece::None)
    }

    pub fn borrow_player(&self) -> Result<&Player, ChessError> {
        match self {
            &Piece::Pawn(ref player) => Ok(player),
            &Piece::Knight(ref player) => Ok(player),
            &Piece::Bishop(ref player) => Ok(player),
            &Piece::Rook(ref player) => Ok(player),
            &Piece::Queen(ref player) => Ok(player),
            &Piece::King(ref player) => Ok(player),
            &Piece::None => Err(ChessError::NoPieceAtPosition),
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
