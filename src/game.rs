pub enum ChessError {
    OutOfBounds,
    NoPieceAtPosition,
    InvalidMove,
    PlayerInCheck,
}

pub enum GameStatus {
    Ongoing,
    Stalemate,
    Check,
    Checkmate
}