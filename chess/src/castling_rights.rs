use crate::{
    board::Board, color::Color, displacement::Displacement, moves::Move, piece::Piece,
    position::Position,
};

#[derive(Clone, Copy)]
pub(super) enum CastlingRightsKind {
    WhiteKingside,
    WhiteQueenside,
    BlackKingside,
    BlackQueenside,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(super) struct CastlingRights([bool; 4]);

impl Default for CastlingRights {
    fn default() -> Self { Self([true, true, true, true]) }
}

impl CastlingRights {
    pub(super) fn rook_positions() -> [(Position, Piece, CastlingRightsKind); 4] {
        [
            (
                Position::WHITE_KINGSIDE_ROOK,
                Piece::Rook(Color::White),
                CastlingRightsKind::WhiteKingside,
            ),
            (
                Position::WHITE_QUEENSIDE_ROOK,
                Piece::Rook(Color::White),
                CastlingRightsKind::WhiteQueenside,
            ),
            (
                Position::BLACK_KINGSIDE_ROOK,
                Piece::Rook(Color::Black),
                CastlingRightsKind::BlackKingside,
            ),
            (
                Position::BLACK_QUEENSIDE_ROOK,
                Piece::Rook(Color::Black),
                CastlingRightsKind::BlackQueenside,
            ),
        ]
    }

    pub(super) fn king_positions() -> [(Position, Piece, CastlingRightsKind, CastlingRightsKind); 2]
    {
        [
            (
                Position::WHITE_KING,
                Piece::King(Color::White),
                CastlingRightsKind::WhiteKingside,
                CastlingRightsKind::WhiteQueenside,
            ),
            (
                Position::BLACK_KING,
                Piece::King(Color::Black),
                CastlingRightsKind::BlackKingside,
                CastlingRightsKind::BlackQueenside,
            ),
        ]
    }

    pub(super) fn get_castling_positions(player: Color) -> (Position, Position, Position) {
        match player {
            Color::White => (
                Position::WHITE_KING,
                Position::WHITE_KINGSIDE_ROOK,
                Position::WHITE_QUEENSIDE_ROOK,
            ),
            Color::Black => (
                Position::BLACK_KING,
                Position::BLACK_KINGSIDE_ROOK,
                Position::BLACK_QUEENSIDE_ROOK,
            ),
        }
    }

    pub(super) fn get_castling_info(
        player: Color,
    ) -> (Position, CastlingRightsKind, CastlingRightsKind) {
        match player {
            Color::White => (
                Position::WHITE_KING,
                CastlingRightsKind::WhiteKingside,
                CastlingRightsKind::WhiteQueenside,
            ),
            Color::Black => (
                Position::BLACK_KING,
                CastlingRightsKind::BlackKingside,
                CastlingRightsKind::BlackQueenside,
            ),
        }
    }

    pub(super) fn update_castling_rights(&mut self, board: &Board) {
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

    pub(super) fn handle_castling_the_rook(&self, mv: &Move, board: &mut Board, player: Color) {
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

    pub(super) fn has_castling_right(&self, right: CastlingRightsKind) -> bool {
        self.0[right as usize]
    }

    pub(super) fn get_fen_str(&self) -> String {
        let mut fen = String::default();
        if self.0[CastlingRightsKind::WhiteKingside as usize] {
            fen.push(Piece::King(Color::White).get_fen_char())
        }
        if self.0[CastlingRightsKind::WhiteQueenside as usize] {
            fen.push(Piece::Queen(Color::White).get_fen_char())
        }
        if self.0[CastlingRightsKind::BlackKingside as usize] {
            fen.push(Piece::King(Color::Black).get_fen_char())
        }
        if self.0[CastlingRightsKind::BlackQueenside as usize] {
            fen.push(Piece::Queen(Color::Black).get_fen_char())
        }
        if fen.is_empty() {
            String::from('-')
        } else {
            fen
        }
    }
}
