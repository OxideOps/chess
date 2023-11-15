use std::hash::Hash;

use crate::{
    board::{Board, Square},
    castling_rights::CastlingRights,
    color::Color,
    displacement::Displacement,
    moves::Move,
    piece::Piece,
    position::Position,
    result::{ChessError, ChessResult},
};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
/// A struct encapsulating the state for the `Board`.
pub(super) struct BoardState {
    pub(super) player: Color,
    board: Board,
    pub(super) castling_rights: CastlingRights,
    pub(super) en_passant_position: Option<Position>,
    pub(super) white_king_position: Position,
    pub(super) black_king_position: Position,
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            player: Color::default(),
            board: Board::default(),
            castling_rights: CastlingRights::default(),
            en_passant_position: None,
            white_king_position: Position::WHITE_KING,
            black_king_position: Position::BLACK_KING,
        }
    }
}

impl BoardState {
    pub(super) fn get_piece(&self, at: &Position) -> Square {
        self.board.get_piece(at)
    }

    fn can_promote_piece(&self, piece: Piece, at: &Position) -> bool {
        piece.is_pawn()
            && ((self.player == Color::White && at.y == 7)
                || (self.player == Color::Black && at.y == 0))
    }

    pub(super) fn move_piece(&mut self, mv: &Move) {
        let mut piece = self.board.take_piece(&mv.from).unwrap();
        if self.can_promote_piece(piece, &mv.to) {
            piece = Piece::Queen(self.player)
        }
        self.board
            .set_piece(&Position::new(mv.to.x, mv.to.y), Some(piece));

        // Update king's position if a king is moved
        if piece == Piece::King(self.player) {
            match self.player {
                Color::White => self.white_king_position = mv.to,
                Color::Black => self.black_king_position = mv.to,
            }
        }

        self.update(mv)
    }

    fn is_square_white(i: usize, j: usize) -> bool {
        (i + j) % 2 == 0
    }

    pub(super) fn is_insufficient_material(&self) -> bool {
        let (
            mut white_bishops,
            mut black_bishops,
            mut white_knights,
            mut black_knights,
            mut other_pieces,
        ) = (0, 0, 0, 0, false);
        let (mut white_square_bishops, mut black_square_bishops) = (0, 0);

        for (i, row) in self.board.iter().enumerate() {
            for (j, piece) in row.iter().enumerate() {
                if let Some(piece) = piece {
                    match piece {
                        Piece::Bishop(Color::White) => {
                            white_bishops += 1;
                            if Self::is_square_white(i, j) {
                                white_square_bishops += 1;
                            }
                        }
                        Piece::Bishop(Color::Black) => {
                            black_bishops += 1;
                            if !Self::is_square_white(i, j) {
                                black_square_bishops += 1;
                            }
                        }
                        Piece::Knight(Color::White) => white_knights += 1,
                        Piece::Knight(Color::Black) => black_knights += 1,
                        Piece::King(_) => (),
                        _ => other_pieces = true,
                    }
                }
            }
        }

        !other_pieces
            && match (white_bishops, black_bishops, white_knights, black_knights) {
                (0, 0, 0..=2, 0..=2) => true,
                (0..=1, 0..=1, 0, 0) if white_square_bishops == black_square_bishops => true,
                _ => false,
            }
    }

    pub(super) fn is_in_bounds(at: &Position) -> ChessResult {
        if at.x > 7 || at.y > 7 {
            Err(ChessError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    pub(super) fn is_piece_some(&self, at: &Position) -> ChessResult {
        if self.board.get_piece(at).is_none() {
            return Err(ChessError::NoPieceAtPosition);
        }
        Ok(())
    }

    fn update(&mut self, mv: &Move) {
        self.castling_rights
            .handle_castling_the_rook(mv, &mut self.board, self.player);
        self.castling_rights.update_castling_rights(&self.board);
        self.handle_capturing_en_passant(&mv.to);
        self.update_en_passant(mv);
        self.player = !self.player;
    }

    pub(super) fn has_piece(&self, position: &Position) -> bool {
        Self::is_in_bounds(position).is_ok() && self.board.get_piece(position).is_some()
    }

    pub(super) fn was_double_move(&self, mv: &Move) -> bool {
        if let Some(Piece::Pawn(player)) = self.board.get_piece(&mv.to) {
            return match player {
                Color::White => mv.from.y == 1 && mv.to.y == 3,
                Color::Black => mv.from.y == 6 && mv.to.y == 4,
            };
        }
        false
    }

    fn handle_capturing_en_passant(&mut self, to: &Position) {
        if Some(*to) == self.en_passant_position {
            self.board.set_piece(
                &(*to - Displacement::get_pawn_advance_vector(self.player)),
                None,
            );
        }
    }

    fn update_en_passant(&mut self, mv: &Move) {
        self.en_passant_position = if self.was_double_move(mv) {
            Some(mv.from + Displacement::get_pawn_advance_vector(self.player))
        } else {
            None
        }
    }
}
