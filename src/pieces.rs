use crate::{board::Board, game::ChessError};

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
        *self != Piece::None
    }

    pub fn take(&mut self) -> Piece {
        mem::replace(self, Piece::None)
    }

    pub fn move_piece_to(&self, board: &mut Board) {

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
