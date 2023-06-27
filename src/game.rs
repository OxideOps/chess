use crate::board::Board;
use crate::moves::Move;
use crate::pieces::{Piece, Position};

/// A result type for chess operations.
/// It's either an `Ok(())`, indicating the operation succeeded,
/// or an `Err(ChessError)` indicating the operation failed with a specific error.
pub type ChessResult = Result<(), ChessError>;

/// An enumeration of possible errors that can occur during a game of chess.
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

/// A struct representing the current status of a chess game.
/// The game can be either Ongoing, in Stalemate, Check, or Checkmate.
#[derive(Default)]
pub enum GameStatus {
    #[default]
    Ongoing,
    Stalemate,
    Check,
    Checkmate,
}

/// A struct representing a game of chess.
/// It includes a board, a game status, a move history and a current move index.
#[derive(Default)]
pub struct Game {
    /// The chessboard for the game.
    board: Board,
    /// The status of the game.
    status: GameStatus,
    /// The history of all moves made during the game.
    move_history: Vec<Move>,
    /// The current move index in the move history.
    current_move: usize,
}

impl Game {
    /// Gets the piece at the given position on the board.
    /// Returns `None` if there is no piece at the position.
    pub fn get_piece(&self, position: &Position) -> Option<Piece> {
        self.board.get_piece(position)
    }

    /// Attempts to move a piece from one position to another on the board.
    /// Returns a `ChessResult`, indicating whether the move was successful.
    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult {
        if let Some(piece) = self.board.get_piece(&from) {
            let mv = Move { from, to };

            self.board.move_piece(&mv)?;
            self.move_history.push(mv);
            self.current_move += 1;
            println!("{} : {}", piece, mv);

            self.board.next_turn();
        }
        Ok(())
    }

    /// Attempts to undo the last move.
    /// Returns a `ChessResult`, indicating whether the undo was successful.
    pub fn undo(&mut self) -> ChessResult {
        self.board
            .move_piece(&self.move_history[self.current_move].inverse())?;
        self.current_move -= 1;
        Ok(())
    }

    /// Attempts to redo the last move.
    /// Returns a `ChessResult`, indicating whether the redo was successful.
    pub fn redo(&mut self) -> ChessResult {
        self.board
            .move_piece(&self.move_history[self.current_move])?;
        self.current_move += 1;
        Ok(())
    }

    /// Attempts to resume the game after it was paused.
    /// Returns a `ChessResult`, indicating whether the game could be resumed.
    pub fn resume(&mut self) -> ChessResult {
        while self.current_move < self.move_history.len() {
            self.redo()?;
        }
        Ok(())
    }
}
