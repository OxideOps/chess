use crate::board::BoardState;
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

#[derive(Default, PartialEq)]
pub enum GameStatus {
    #[default]
    Ongoing,
    Stalemate,
    Check,
    Checkmate,
}

#[derive(Default, PartialEq)]
pub struct Game {
    board_state: BoardState,
    status: GameStatus,
}

#[derive(Clone, Copy)]
pub enum CastlingRights {
    WhiteKingside,
    WhiteQueenside,
    BlackKingside,
    BlackQueenside,
}

impl CastlingRights {
    pub const WHITE_KING: Position = Position { x: 4, y: 0 };
    pub const BLACK_KING: Position = Position { x: 4, y: 7 };
    pub const WHITE_KINGSIDE_ROOK: Position = Position { x: 7, y: 0 };
    pub const WHITE_QUEENSIDE_ROOK: Position = Position { x: 0, y: 0 };
    pub const BLACK_KINGSIDE_ROOK: Position = Position { x: 7, y: 7 };
    pub const BLACK_QUEENSIDE_ROOK: Position = Position { x: 0, y: 7 };
}

impl Game {
    pub fn get_piece(&self, position: &Position) -> Option<Piece> {
        self.board_state.board.get_piece(position)
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult {
        if let Some(piece) = self.board_state.board.get_piece(&from) {
            let mv = Move { from, to };

            self.board_state.move_piece(&mv)?;
            println!("{} : {}", piece, mv);

            self.board_state.next_turn();
        }
        Ok(())
    }
}
