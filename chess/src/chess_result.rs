use thiserror::Error;

pub type ChessResult = Result<(), ChessError>;
#[derive(Debug, Error)]
pub enum ChessError {
    #[error("Out of bounds")]
    OutOfBounds,
    #[error("No piece at position")]
    NoPieceAtPosition,
    #[error("Invalid move")]
    InvalidMove,
    #[error("Moved after game drawn")]
    GameIsInDraw,
}
