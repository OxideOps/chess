use crate::{board::Board, Color, Piece};

pub(super) struct PieceCount {
    white_bishops: u8,
    black_bishops: u8,
    white_knights: u8,
    black_knights: u8,
}

impl PieceCount {
    fn new() -> Self {
        PieceCount {
            white_bishops: 0,
            black_bishops: 0,
            white_knights: 0,
            black_knights: 0,
        }
    }

    pub fn is_draw(board: &Board) -> bool {
        let mut count = Self::new();

        for row in board.iter() {
            for piece in row.iter() {
                match piece {
                    Some(Piece::Bishop(Color::White)) => count.white_bishops += 1,
                    Some(Piece::Bishop(Color::Black)) => count.black_bishops += 1,
                    Some(Piece::Knight(Color::White)) => count.white_knights += 1,
                    Some(Piece::Knight(Color::Black)) => count.black_knights += 1,
                    Some(Piece::King(_)) => (),
                    Some(_) => return false,
                    None => (),
                }
            }
        }

        matches!(
            (
                count.white_bishops,
                count.black_bishops,
                count.white_knights,
                count.black_knights,
            ),
            (0, 0, 0, 0) | (1, 0, 0, 0) | (0, 1, 0, 0) | (0, 0, 1, 0) | (0, 0, 0, 1)
        )
    }
}
