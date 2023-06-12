use crate::game::ChessError;
use crate::pieces::{Piece, Position};

pub struct Board {
    squares: [[Option<Box<dyn Piece>>; 8]; 8],
}

impl Board {
    pub fn new() -> Self {
        let squares: [[Option<Box<dyn Piece>>; 8]; 8] = Default::default();
        Self { squares }
    }

    pub fn get_piece(&self, position: Position) -> Result<Option<&dyn Piece>, ChessError> {
        if position.x > 7 || position.y > 7 {
            return Err(ChessError::OutOfBounds);
        }
        Ok(self.squares[position.x][position.y].as_deref())
    }

    pub fn can_move(&self, piece: &dyn Piece, from: Position, to: Position) -> bool {
        false
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> Result<(), ChessError> {
        let piece = self.get_piece(from)?.ok_or(ChessError::NoPieceAtPosition)?;

        if self.can_move(piece, from, to) {
            self.squares[to.x][to.y] = self.squares[from.x][from.y].take();
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }
}
