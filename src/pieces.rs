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

    pub fn can_move(
        &self,
        board: &mut Board,
        to: Position,
        from: Position,
    ) -> Result<bool, ChessError> {
        let to_piece = board.borrow_piece_at_mut(to)?;
        let from_piece = board.borrow_piece_at_mut(from)?;

        //Do logic for each piece
        match *self {
            Piece::Pawn(..) => {
                //Err(Piece::PawnError::SomeError)
                return Ok(false);
            }
            Piece::Knight(..) => return Ok(false),
            Piece::Bishop(..) => return Ok(false),
            Piece::Rook(..) => {
                //Err(Piece::RookError::SomeOtherError)
                return Ok(true);
            }
            Piece::Queen(..) => return Ok(false),
            Piece::King(..) => return Ok(false),
            Piece::None => return Ok(false),
        };
        Ok(true)
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
