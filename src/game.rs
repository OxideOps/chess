use crate::board::Board;
use crate::moves::Move;
use crate::pieces::{Piece, Position};

pub type ChessResult = Result<(), ChessError>;
#[derive(Debug)]
pub enum ChessError {
    OutOfBounds,
    NoPieceAtPosition,
    InvalidMove,
    OwnPieceInDestination,
    PlayerInCheck,
    Checkmate,
    Stalemate,
    InvalidPromotion,
    NotPlayersTurn,
    EmptyPieceMove,
}

#[derive(Default)]
pub enum GameStatus {
    #[default]
    Ongoing,
    Stalemate,
    Check,
    Checkmate,
}

#[derive(Default)]
pub struct Game {
    board: Board,
    status: GameStatus,
}

impl Game {
    pub fn get_piece(&self, position: &Position) -> Option<Piece> {
        self.board.get_piece(position)
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult {
        if let Some(piece) = self.board.get_piece(&from) {
            let mv = Move { from, to };

            self.board.move_piece(&mv)?;
            println!("{} : {}", piece, mv);

            self.board.next_turn();
        }
        Ok(())
    }
}
