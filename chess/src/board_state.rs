use crate::board::{Board, Square};
use crate::castling_rights::CastlingRights;
use crate::displacement::Displacement;
use crate::chess_result::{ChessError, ChessResult};
use crate::moves::Move;
use crate::pieces::{Color, Piece, Position};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Default, Hash)]
/// A struct encapsulating the state for the `Board`.
pub struct BoardState {
    pub player: Color,
    board: Board,
    pub castling_rights: CastlingRights,
    pub en_passant_position: Option<Position>,
}

impl BoardState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_piece(&self, at: &Position) -> Square {
        self.board.get_piece(at)
    }

    fn can_promote_piece(&self, piece: Piece, at: &Position) -> bool {
        piece.is_pawn()
            && ((self.player == Color::White && at.y == 7)
                || (self.player == Color::Black && at.y == 0))
    }

    pub fn move_piece(&mut self, mv: &Move) {
        let mut piece = self.board.take_piece(&mv.from).unwrap();
        if self.can_promote_piece(piece, &mv.to) {
            piece = Piece::Queen(self.player)
        }
        self.board
            .set_piece(&Position::new(mv.to.x, mv.to.y), Some(piece));
        self.update(mv)
    }

    pub fn is_in_bounds(at: &Position) -> ChessResult {
        if at.x > 7 || at.y > 7 {
            Err(ChessError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    pub fn is_piece_some(&self, at: &Position) -> ChessResult {
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

    pub fn has_piece(&self, position: &Position) -> bool {
        Self::is_in_bounds(position).is_ok() && self.board.get_piece(position).is_some()
    }

    pub fn was_double_move(&self, mv: &Move) -> bool {
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

    pub fn get_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }
}
