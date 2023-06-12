pub enum ChessError {
    OutOfBounds,
    NoPieceAtPosition,
    InvalidMove,
    PlayerInCheck,
    NoValidMoves,
}

pub enum GameStatus {
    Ongoing,
    Stalemate,
    Check,
    Checkmate,
}
