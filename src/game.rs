pub enum ChessError {
    OutOfBounds,
    NoPieceAtPosition,
    InvalidMove,
    OwnPieceInDestination,
    PlayerInCheck,
}

pub enum GameStatus {
    Ongoing,
    Stalemate,
    Check,
    Checkmate
}