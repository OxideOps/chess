use crate::pieces::Piece;
use crate::game::ChessError;

pub struct Board {
    squares: [[Option<Box<dyn Piece>>; 8]; 8],
}

impl Board {
    pub fn new() -> Self {
        let squares: [[Option<Box<dyn Piece>>; 8]; 8] = Default::default();
        Self { squares }
    }

    pub fn get_piece_at(&self, position: (usize, usize)) -> Option<&dyn Piece> {
        let (x, y) = position;
        self.squares[x][y].as_deref()
    }

    pub fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> Result<(), ChessError> {
        if let &Some(ref piece) = &self.squares[from.0][from.1] {
            if piece.can_move(from, to) {
                let piece = self.squares[from.0][from.1].take();
                self.squares[to.0][to.1] = piece;
                Ok(())
            } else {
                Err(ChessError::InvalidMove)
            }
        } else {
            Err(ChessError::NoPieceAtPosition)
        }
    }
}
