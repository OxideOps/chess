pub trait Piece {
    fn color(&self) -> Color;
    fn can_move(&self, from: (usize, usize), to: (usize, usize)) -> bool;
    // Add other common methods here
}

pub struct Pawn {
    color: Color,
    has_moved: bool,
}

impl Piece for Pawn {
    fn color(&self) -> Color {
        self.color
    }

    fn can_move(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        // Implement pawn-specific move logic
        // Check if the move is valid for a pawn
        // Return true or false accordingly
    }

    // Implement other methods specific to the pawn
}

// Implement other chess piece types similarly (e.g., Rook, Knight, Bishop, etc.)

pub struct Board {
    // Define the chess board and other board-related fields
    // ...
}

impl Board {
    // Implement board-related methods, such as move_piece, get_piece_at, etc.
    // ...
}

#[derive(Clone, Copy)]
pub enum Color {
    White,
    Black
}

