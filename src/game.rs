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
