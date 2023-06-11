use crate::pieces::{Piece, Color};
use crate::game::GameStatus;

pub struct Board {
    board: [[Option<Piece>; 8]; 8],
    current_player: Color,
    game_status: GameStatus,
    white_can_castle: bool,
    black_can_castle: bool,
}

impl Board {
    pub fn new() -> Self {
        let mut board = [[None; 8]; 8];

        // Initialize white pawns
        for i in 0..8 {
            board[1][i] = Some(Piece::Pawn(Color::White));
        }

        // Initialize black pawns
        for i in 0..8 {
            board[6][i] = Some(Piece::Pawn(Color::Black));
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
            board[0][i] = Some(back_row[i](Color::White));
            board[7][i] = Some(back_row[i](Color::Black));
        }

        Self { 
            board,
            current_player: Color::White,
            game_status: GameStatus::Ongoing,
            white_can_castle: false,
            black_can_castle: false,
        }
    }

    pub fn get_piece_at(&self, position: (usize, usize)) -> Option<&Piece> {
        
    }

    pub fn move_piece(&mut self, from: (usize, usize), to: (usize, usize)) -> Result<(), ChessError> {
        // move a piece from one square to another
    }

    pub fn is_check(&self, color: Color) -> bool {
        // check if the given player is in check
    }

    pub fn is_checkmate(&self, color: Color) -> bool {
        // check if the given player is in checkmate
    }
}