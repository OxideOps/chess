use crate::pieces::{Piece, Color};
use crate::game::{GameStatus, ChessError};

pub struct Board {
    squares: [[Option<Piece>; 8]; 8],
    current_player: Color,
    game_status: GameStatus,
    white_can_castle: bool,
    black_can_castle: bool,
}

impl Board {
    fn new() -> Self {
        let mut squares = [[None; 8]; 8];

        // Initialize white pawns
        for i in 0..8 {
            squares[1][i] = Some(Piece::Pawn(Color::White));
        }

        // Initialize black pawns
        for i in 0..8 {
            squares[6][i] = Some(Piece::Pawn(Color::Black));
        }

        // Initialize the other white and black pieces
        let back_row = [
            Piece::Rook,
            Piece::Knight,
            Piece::Bishop,
            Piece::Queen,
            Piece::King,
            Piece::Bishop,
            Piece::Knight,
            Piece::Rook,
        ];
        for i in 0..8 {
            squares[0][i] = Some(back_row[i](Color::White));
            squares[7][i] = Some(back_row[i](Color::Black));
        }

        Self { 
            squares,
            current_player: Color::White,
            game_status: GameStatus::Ongoing,
            white_can_castle: false,
            black_can_castle: false,
        }
    }

    fn get_piece(&self, position: (usize, usize)) -> Result<Option<&Piece>, ChessError> {
        if position.0 > 7 || position.1 > 7 {
            return Err(ChessError::OutOfBounds)
        }
        let piece = self.squares[position.0][position.1].as_ref();

        if piece.is_none() {
            return Err(ChessError::NoPieceAtPosition)
        }
        Ok(piece)
    }

    fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> Result<(), ChessError> {
        match self.get_piece(from)? {
            Some(piece) => {
                if self.is_valid_move(piece, from, to) {
                    self.squares[to.0][to.1] = self.squares[from.0][from.1].take();
                    Ok(())
                } else {
                    Err(ChessError::InvalidMove)
                }
            },
            None => Err(ChessError::NoPieceAtPosition),
        }
    }
    
    fn is_valid_move(&self, piece: &Piece, from: (usize, usize), to: (usize, usize)) -> bool {
        match *piece {
            Piece::Pawn(color) => {
                true
            },
            Piece::Knight(color) => {
                true
            },
            _ => true,  
        }
    }
    

    fn is_check(&self, color: Color) -> bool {
        true
    }

    fn is_checkmate(&self, color: Color) -> bool {
        true
    }
}