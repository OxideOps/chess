#[derive(Clone, Copy, PartialEq)]
pub enum Piece {
    Pawn(Color),
    Rook(Color),
    Knight(Color),
    Bishop(Color),
    Queen(Color),
    King(Color),
}

impl Piece {
    fn color(&self) -> Color {
        match *self {
            Piece::Pawn(color) => color,
            Piece::Rook(color) => color,
            Piece::Knight(color) => color,
            Piece::Bishop(color) => color,
            Piece::Queen(color) => color,
            Piece::King(color) => color,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Color {
    White,
    Black
}

