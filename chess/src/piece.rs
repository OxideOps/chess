use std::fmt;

use crate::{color::Color, displacement::Displacement};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Piece {
    Pawn(Color),
    Knight(Color),
    Bishop(Color),
    Rook(Color),
    Queen(Color),
    King(Color),
}

impl Piece {
    pub(super) fn is_pawn(self) -> bool { matches!(self, Piece::Pawn(..)) }

    pub(super) fn get_player(self) -> Color {
        match self {
            Self::Pawn(player)
            | Self::Knight(player)
            | Self::Bishop(player)
            | Self::Rook(player)
            | Self::Queen(player)
            | Self::King(player) => player,
        }
    }

    pub(super) fn get_vectors(self) -> &'static [Displacement] {
        match self {
            Self::Pawn(..) => panic!("Try calling `Displacement::get_pawn_*_vector()` instead"),
            Self::Rook(..) => Displacement::get_rook_vectors(),
            Self::Bishop(..) => Displacement::get_bishop_vectors(),
            Self::Knight(..) => Displacement::get_knight_vectors(),
            Self::Queen(..) => Displacement::get_queen_vectors(),
            Self::King(..) => Displacement::get_king_vectors(),
        }
    }

    pub(super) fn can_snipe(self) -> bool {
        matches!(self, Self::Bishop(..) | Self::Rook(..) | Self::Queen(..))
    }

    pub(super) fn get_fen_char(&self) -> char {
        let c = match self {
            Piece::Pawn(..) => 'p',
            Piece::Bishop(..) => 'b',
            Piece::Knight(..) => 'n',
            Piece::Rook(..) => 'r',
            Piece::King(..) => 'k',
            Piece::Queen(..) => 'q',
        };
        match self.get_player() {
            Color::White => c.to_uppercase().next().unwrap(),
            Color::Black => c,
        }
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let piece = match self {
            Piece::Pawn(Color::White) => "♟",
            Piece::Knight(Color::White) => "♞",
            Piece::Bishop(Color::White) => "♝",
            Piece::Rook(Color::White) => "♜",
            Piece::Queen(Color::White) => "♛",
            Piece::King(Color::White) => "♚",
            Piece::Pawn(Color::Black) => "♙",
            Piece::Knight(Color::Black) => "♘",
            Piece::Bishop(Color::Black) => "♗",
            Piece::Rook(Color::Black) => "♖",
            Piece::Queen(Color::Black) => "♕",
            Piece::King(Color::Black) => "♔",
        };
        write!(f, "{}", piece)
    }
}
