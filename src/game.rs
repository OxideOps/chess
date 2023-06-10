pub enum ChessError {
    OutOfBounds,
    NoPieceAtPosition,
    InvalidMove,
    OwnPieceInDestination,
    PlayerInCheck()
}