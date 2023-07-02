use crate::{
    board::Board,
    displacement::Displacement,
    moves::Move,
    pieces::{Piece, Player, Position},
};

#[derive(Clone, Copy)]
pub enum CastlingRightsKind {
    WhiteKingside,
    WhiteQueenside,
    BlackKingside,
    BlackQueenside,
}

#[derive(Clone)]
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

    pub fn update_castling_rights(&mut self, board: &Board) {
        for (position, piece, rights) in CastlingRights::rook_positions() {
            if board.get_piece(&position) != Some(piece) {
                self.0[rights as usize] = false;
            }
        }

        for (position, piece, kingside_rights, queenside_rights) in CastlingRights::king_positions()
        {
            if board.get_piece(&position) != Some(piece) {
                self.0[kingside_rights as usize] = false;
                self.0[queenside_rights as usize] = false;
            }
        }
    }

    pub fn handle_castling_the_rook(&self, mv: &Move, board: &mut Board, player: Player) {
        let (king, kingside_rook, queenside_rook) = CastlingRights::get_castling_positions(player);

        if mv.from == king {
            if mv.to == king + Displacement::RIGHT * 2 {
                let rook = board.take_piece(&kingside_rook);
                board.set_piece(&(kingside_rook + Displacement::LEFT * 2), rook);
            } else if mv.to == king + Displacement::LEFT * 2 {
                let rook = board.take_piece(&queenside_rook);
                board.set_piece(&(queenside_rook + Displacement::RIGHT * 3), rook);
            }
        }
    }

    pub fn has_castling_right(&self, right: CastlingRightsKind) -> bool {
        self.0[right as usize]
    }
}
