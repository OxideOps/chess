use crate::{color::Color, piece::Piece, position::Position};
use std::hash::Hash;

const BOARD_SIZE: usize = 8;

pub(super) type Square = Option<Piece>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Board([[Square; BOARD_SIZE]; BOARD_SIZE]);

impl Default for Board {
    fn default() -> Self {
        let mut squares = [[None; BOARD_SIZE]; BOARD_SIZE];

        // Initialize pawns
        for i in 0..8 {
            squares[1][i] = Some(Piece::Pawn(Color::White));
            squares[6][i] = Some(Piece::Pawn(Color::Black));
        }

        // Initialize the other pieces
        squares[0] = Self::get_back_rank(Color::White);
        squares[BOARD_SIZE - 1] = Self::get_back_rank(Color::Black);

        Self(squares)
    }
}

impl Board {
    pub(super) fn new() -> Self {
        Self::default()
    }

    fn get_back_rank(player: Color) -> [Square; 8] {
        [
            Some(Piece::Rook(player)),
            Some(Piece::Knight(player)),
            Some(Piece::Bishop(player)),
            Some(Piece::Queen(player)),
            Some(Piece::King(player)),
            Some(Piece::Bishop(player)),
            Some(Piece::Knight(player)),
            Some(Piece::Rook(player)),
        ]
    }

    pub(super) fn get_piece(&self, at: &Position) -> Square {
        self.0[at.y][at.x]
    }

    pub(super) fn set_piece(&mut self, at: &Position, square: Square) {
        self.0[at.y][at.x] = square;
    }

    pub(super) fn take_piece(&mut self, from: &Position) -> Square {
        self.0[from.y][from.x].take()
    }
}
