use crate::{board::Board, Color, Piece};

pub(super) struct PieceCount {
    white_bishops: usize,
    black_bishops: usize,
    white_knights: usize,
    black_knights: usize,
    other_pieces: bool,
}

impl PieceCount {
    fn new() -> Self {
        PieceCount {
            white_bishops: 0,
            black_bishops: 0,
            white_knights: 0,
            black_knights: 0,
            other_pieces: false,
        }
    }

    pub fn from_board(board: &Board) -> Self {
        let mut count = PieceCount::new();

        for row in board.iter() {
            for piece in row.iter() {
                match piece {
                    Some(Piece::Bishop(Color::White)) => count.white_bishops += 1,
                    Some(Piece::Bishop(Color::Black)) => count.black_bishops += 1,
                    Some(Piece::Knight(Color::White)) => count.white_knights += 1,
                    Some(Piece::Knight(Color::Black)) => count.black_knights += 1,
                    Some(Piece::King(_)) => (),
                    Some(_) => count.other_pieces = true,
                    None => (),
                }
            }
        }

        count
    }

    pub fn is_draw(&self) -> bool {
        if self.other_pieces {
            return false;
        }

        match (
            self.white_bishops,
            self.black_bishops,
            self.white_knights,
            self.black_knights,
        ) {
            (0, 0, 0, 0) => true,
            (0, 0, 1, 0) | (1, 0, 0, 0) | (0, 0, 0, 1) | (0, 1, 0, 0) => true,
            _ => false,
        }
    }
}
