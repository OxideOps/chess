pub trait Piece: {
    fn color(&self) -> Color;
    fn can_move(&self, from: (usize, usize), to: (usize, usize)) -> bool;
    // Add other common methods here
}

#[derive(Copy, Clone)]
pub struct Pawn {
    color: Color,
    has_moved: bool,
}

impl Piece for Pawn {
    fn color(&self) -> Color {
        self.color
    }

    fn can_move(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        self.has_moved
    }
}

#[derive(Clone, Copy)]
pub enum Color {
    White,
    Black
}

