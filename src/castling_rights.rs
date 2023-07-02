use crate::pieces::{Piece, Player, Position};

#[derive(Clone, Copy)]
pub enum CastlingRights {
    WhiteKingside,
    WhiteQueenside,
    BlackKingside,
    BlackQueenside,
}

impl CastlingRights {
    pub const WHITE_KING: Position = Position { x: 4, y: 0 };
    pub const BLACK_KING: Position = Position { x: 4, y: 7 };
    pub const WHITE_KINGSIDE_ROOK: Position = Position { x: 7, y: 0 };
    pub const WHITE_QUEENSIDE_ROOK: Position = Position { x: 0, y: 0 };
    pub const BLACK_KINGSIDE_ROOK: Position = Position { x: 7, y: 7 };
    pub const BLACK_QUEENSIDE_ROOK: Position = Position { x: 0, y: 7 };

    pub fn rook_positions() -> [(Position, Piece, CastlingRights); 4] {
        [
            (
                CastlingRights::WHITE_KINGSIDE_ROOK,
                Piece::Rook(Player::White),
                CastlingRights::WhiteKingside,
            ),
            (
                CastlingRights::WHITE_QUEENSIDE_ROOK,
                Piece::Rook(Player::White),
                CastlingRights::WhiteQueenside,
            ),
            (
                CastlingRights::BLACK_KINGSIDE_ROOK,
                Piece::Rook(Player::Black),
                CastlingRights::BlackKingside,
            ),
            (
                CastlingRights::BLACK_QUEENSIDE_ROOK,
                Piece::Rook(Player::Black),
                CastlingRights::BlackQueenside,
            ),
        ]
    }

    pub fn king_positions() -> [(Position, Piece, CastlingRights, CastlingRights); 2] {
        [
            (
                CastlingRights::WHITE_KING,
                Piece::King(Player::White),
                CastlingRights::WhiteKingside,
                CastlingRights::WhiteQueenside,
            ),
            (
                CastlingRights::BLACK_KING,
                Piece::King(Player::Black),
                CastlingRights::BlackKingside,
                CastlingRights::BlackQueenside,
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

        
        
    pub fn get_castling_info(player: Player) -> (Position, CastlingRights, CastlingRights) {
        match player {
            Player::White => (
                CastlingRights::WHITE_KING,
                CastlingRights::WhiteKingside,
                CastlingRights::WhiteQueenside,
            ),
            Player::Black => (
                CastlingRights::BLACK_KING,
                CastlingRights::BlackKingside,
                CastlingRights::BlackQueenside,
            ),
        }
    }
}
