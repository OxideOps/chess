use std::collections::HashSet;

use crate::{pieces::{Piece, Player, Position}, board::BoardState, displacement::Displacement, moves::Move};

#[derive(Clone, Copy)]
pub enum CastlingRightsKind {
    WhiteKingside,
    WhiteQueenside,
    BlackKingside,
    BlackQueenside,
}

pub struct CastlingRights([bool; 4]);

impl Default for CastlingRights {
    fn default() -> Self {
        Self([true, true, true, true])
    }
}

impl CastlingRights {
    pub const WHITE_KING: Position = Position { x: 4, y: 0 };
    pub const BLACK_KING: Position = Position { x: 4, y: 7 };
    pub const WHITE_KINGSIDE_ROOK: Position = Position { x: 7, y: 0 };
    pub const WHITE_QUEENSIDE_ROOK: Position = Position { x: 0, y: 0 };
    pub const BLACK_KINGSIDE_ROOK: Position = Position { x: 7, y: 7 };
    pub const BLACK_QUEENSIDE_ROOK: Position = Position { x: 0, y: 7 };

    pub fn new() -> Self {
        Self::default()
    }

    pub fn rook_positions() -> [(Position, Piece, CastlingRightsKind); 4] {
        [
            (
                CastlingRights::WHITE_KINGSIDE_ROOK,
                Piece::Rook(Player::White),
                CastlingRightsKind::WhiteKingside,
            ),
            (
                CastlingRights::WHITE_QUEENSIDE_ROOK,
                Piece::Rook(Player::White),
                CastlingRightsKind::WhiteQueenside,
            ),
            (
                CastlingRights::BLACK_KINGSIDE_ROOK,
                Piece::Rook(Player::Black),
                CastlingRightsKind::BlackKingside,
            ),
            (
                CastlingRights::BLACK_QUEENSIDE_ROOK,
                Piece::Rook(Player::Black),
                CastlingRightsKind::BlackQueenside,
            ),
        ]
    }

    pub fn king_positions() -> [(Position, Piece, CastlingRightsKind, CastlingRightsKind); 2] {
        [
            (
                CastlingRights::WHITE_KING,
                Piece::King(Player::White),
                CastlingRightsKind::WhiteKingside,
                CastlingRightsKind::WhiteQueenside,
            ),
            (
                CastlingRights::BLACK_KING,
                Piece::King(Player::Black),
                CastlingRightsKind::BlackKingside,
                CastlingRightsKind::BlackQueenside,
            ),
        ]
    }

    pub fn get_castling_positions(player: Player) -> (Position, Position, Position) {
        match player {
            Player::White => (
                CastlingRights::WHITE_KING,
                CastlingRights::WHITE_KINGSIDE_ROOK,
                CastlingRights::WHITE_QUEENSIDE_ROOK,
            ),
            Player::Black => (
                CastlingRights::BLACK_KING,
                CastlingRights::BLACK_KINGSIDE_ROOK,
                CastlingRights::BLACK_QUEENSIDE_ROOK,
            ),
        }
    }

    pub fn get_castling_info(player: Player) -> (Position, CastlingRightsKind, CastlingRightsKind) {
        match player {
            Player::White => (
                CastlingRights::WHITE_KING,
                CastlingRightsKind::WhiteKingside,
                CastlingRightsKind::WhiteQueenside,
            ),
            Player::Black => (
                CastlingRights::BLACK_KING,
                CastlingRightsKind::BlackKingside,
                CastlingRightsKind::BlackQueenside,
            ),
        }
    }

    pub fn update_castling_rights(&mut self, state: &BoardState) {
        for &(position, piece, rights) in CastlingRights::rook_positions().as_ref() {
            if state.get_piece(&position) != Some(piece) {
                self.0[rights as usize] = false;
            }
        }

        for &(position, piece, kingside_rights, queenside_rights) in
            CastlingRights::king_positions().as_ref()
        {
            if state.get_piece(&position) != Some(piece) {
                self.0[kingside_rights as usize] = false;
                self.0[queenside_rights as usize] = false;
            }
        }
    }

    pub fn add_castling_moves(&self, valid_moves: &mut HashSet<Move>, state: &BoardState) {
        let (king_square, kingside, queenside) =
            CastlingRights::get_castling_info(state.player);

        if self.0[kingside as usize]
            && !(1..=2).any(|i| state.has_piece(&(king_square + Displacement::RIGHT * i)))
        {
            valid_moves.insert(Move {
                from: king_square,
                to: king_square + Displacement::RIGHT * 2,
            });
        }

        if self.0[queenside as usize]
            && !(1..=3).any(|i| state.has_piece(&(king_square + Displacement::LEFT * i)))
        {
            valid_moves.insert(Move {
                from: king_square,
                to: king_square + Displacement::LEFT * 2,
            });
        }
    }

    pub fn handle_castling_the_rook(&mut self, mv: &Move, state: &mut BoardState) {
        let (king, kingside_rook, queenside_rook) =
            CastlingRights::get_castling_positions(state.player);

        if mv.from == king {
            if mv.to == king + Displacement::RIGHT * 2 {
                let rook = state.take_piece(&kingside_rook);
                state.set_piece(&(kingside_rook + Displacement::LEFT * 2), rook);
            } else if mv.to == king + Displacement::LEFT * 2 {
                let rook = state.take_piece(&queenside_rook);
                state.set_piece(&(queenside_rook + Displacement::RIGHT * 3), rook);
            }
        }
    }


}
