use crate::board::Board;
use crate::moves::Move;
use crate::pieces::{Piece, Position};

pub type ChessResult = Result<(), ChessError>;
#[derive(Debug)]
pub enum ChessError {
    OutOfBounds,           // Position is out of board bounds
    NoPieceAtPosition,     // There's no piece at the specified position
    InvalidMove,           // The move is not valid for the piece
    OwnPieceInDestination, // There's a piece of the same color in the destination position
    PlayerInCheck,         // The current player is in check
    Checkmate,             // The current player is in checkmate
    Stalemate,             // The current player is in stalemate
    InvalidPromotion,      // Invalid pawn promotion
    NotPlayersTurn,        // Trying to move opponent's piece
    EmptyPieceMove,        // Trying to move an empty piece
}

pub enum GameStatus {
    Ongoing,
    Stalemate,
    Check,
    Checkmate,
}

pub struct Game {
    board: Board,
    status: GameStatus,
}

impl Game {
    pub fn new() -> Self {
        Self {
            board: Board::new(),
            status: GameStatus::Ongoing,
        }
    }

    pub fn get_piece(&self, position: &Position) -> Option<Piece> {
        self.board.get_piece(position)
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult {
        let mv = Move { from, to };
        let piece = self.board.get_piece(&from).unwrap();

        self.board.move_piece(&mv)?;
        println!("{} : {}", piece, mv);

        self.board.next_turn();
        Ok(())
    }
}
