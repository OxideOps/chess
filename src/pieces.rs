pub trait Piece: {
    fn color(&self) -> Color;
    fn position(&self) -> Position;
    fn can_move(&self, from: Position, to: Position) -> bool;
    // Add other common methods here
}

#[derive(Copy, Clone)]
pub struct Pawn {
    color: Color,
    position: Position,
}

impl Piece for Pawn {
    fn color(&self) -> Color {
        self.color
    }

    fn can_move(&self, from: Position, to: Position) -> bool {
        //check if there is a valid move
        false
    }

    fn position(&self) -> Position {
        self.position
    }
}

#[derive(Clone, Copy)]
pub enum Color {
    White,
    Black
}

#[derive(Clone, Copy)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

