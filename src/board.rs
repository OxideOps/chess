use crate::displacement::Displacement;
use crate::game::{ChessError, ChessResult};
use crate::moves::Move;
use crate::pieces::{Piece, Player, Position};
use std::collections::HashSet;

const BOARD_SIZE: usize = 8;

enum CastleRights {
    WhiteKingside,
    WhiteQueenside,
    BlackKingside,
    BlackQueenside,
}

impl CastleRights {
    pub const WHITE_KING: Position = Position { x: 4, y: 0 };
    pub const BLACK_KING: Position = Position { x: 4, y: 7 };
    pub const WHITE_KINGSIDE_ROOK: Position = Position { x: 7, y: 0 };
    pub const WHITE_QUEENSIDE_ROOK: Position = Position { x: 0, y: 0 };
    pub const BLACK_KINGSIDE_ROOK: Position = Position { x: 7, y: 7 };
    pub const BLACK_QUEENSIDE_ROOK: Position = Position { x: 0, y: 7 };
}

pub struct Board {
    squares: [[Option<Piece>; BOARD_SIZE]; BOARD_SIZE],
    moves: HashSet<Move>,
    pub player: Player,
    castle_rights: [bool; 4],
    en_passant_square: Option<Position>,
}

impl Board {
    pub fn new() -> Self {
        let mut squares = [[None; BOARD_SIZE]; BOARD_SIZE];

        // Initialize white pawns
        for i in 0..8 {
            squares[1][i] = Some(Piece::Pawn(Player::White));
        }

        // Initialize black pawns
        for i in 0..8 {
            squares[6][i] = Some(Piece::Pawn(Player::Black));
        }

        // Initialize the other white and black pieces
        squares[0] = Self::get_back_rank(Player::White);
        squares[BOARD_SIZE - 1] = Self::get_back_rank(Player::Black);

        let mut board = Self {
            squares,
            moves: HashSet::new(),
            player: Player::White,
            castle_rights: [true, true, true, true],
            en_passant_square: None,
        };
        board.add_moves();
        board
    }

    fn get_back_rank(player: Player) -> [Option<Piece>; 8] {
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

    pub fn get_piece(&self, from: &Position) -> Option<Piece> {
        self.squares[from.y][from.x]
    }

    fn set_piece(&mut self, at: &Position, piece: Option<Piece>) {
        self.squares[at.y][at.x] = piece;
    }

    pub fn take_piece(&mut self, from: &Position) -> Option<Piece> {
        self.squares[from.y][from.x].take()
    }

    fn is_in_bounds(at: &Position) -> ChessResult {
        if at.x > 7 || at.y > 7 {
            Err(ChessError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    fn is_piece_some(&self, at: &Position) -> ChessResult {
        if let None = self.get_piece(at) {
            Err(ChessError::NoPieceAtPosition)
        } else {
            Ok(())
        }
    }

    fn is_move_valid(&self, mv: &Move) -> ChessResult {
        Self::is_in_bounds(&mv.from)?;
        Self::is_in_bounds(&mv.to)?;
        self.is_piece_some(&mv.from)?;

        if self.moves.contains(mv) {
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }

    fn can_promote_piece(&self, at: &Position) -> bool {
        (self.player == Player::White && at.y == 7) || (self.player == Player::Black && at.y == 0)
    }

    pub fn move_piece(&mut self, mv: &Move) -> ChessResult {
        self.is_move_valid(mv)?;

        let mut piece = self.take_piece(&mv.from).unwrap();
        if piece.is_pawn() && self.can_promote_piece(&mv.to) {
            piece = Piece::Queen(self.player)
        }
        self.squares[mv.to.y][mv.to.x] = Some(piece);
        self.handle_castling_the_rook(mv);
        self.handle_capturing_en_passant(&mv.to);
        self.update_castling_rights();
        self.update_en_passant(&mv);
        Ok(())
    }

    pub fn next_turn(&mut self) {
        self.player = !self.player;
        self.add_moves();
    }

    fn can_double_move(&self, from: &Position) -> bool {
        if let Piece::Pawn(player) = self.get_piece(from).unwrap() {
            return match player {
                Player::White => from.y == 1,
                Player::Black => from.y == 6,
            };
        }
        false
    }

    fn add_pawn_advance_moves(&mut self, from: Position) {
        let v = Displacement::get_pawn_advance_vector(self.player);
        let mut to = from + v;
        if Self::is_in_bounds(&to).is_ok() && self.get_piece(&to).is_none() {
            self.moves.insert(Move { from, to });
            to += v;
            if self.get_piece(&to).is_none() {
                if self.can_double_move(&from) {
                    self.moves.insert(Move { from, to });
                }
            }
        }
    }

    fn add_pawn_capture_moves(&mut self, from: Position) {
        for &v in Displacement::get_pawn_capture_vectors(self.player) {
            let to = from + v;
            if Self::is_in_bounds(&to).is_ok() {
                if let Some(piece) = self.get_piece(&to) {
                    if piece.get_player() != self.player {
                        self.moves.insert(Move { from, to });
                    }
                }
                if Some(to) == self.en_passant_square {
                    self.moves.insert(Move { from, to });
                }
            }
        }
    }

    fn add_moves_for_piece(&mut self, from: Position) {
        if let Some(piece) = self.get_piece(&from) {
            if piece.get_player() == self.player {
                if piece.is_pawn() {
                    self.add_pawn_advance_moves(from);
                    self.add_pawn_capture_moves(from);
                } else {
                    for &v in piece.get_vectors() {
                        let mut to = from + v;
                        while Self::is_in_bounds(&to).is_ok() {
                            if let Some(piece) = self.get_piece(&to) {
                                if piece.get_player() != self.player {
                                    self.moves.insert(Move { from, to });
                                }
                                break;
                            }
                            self.moves.insert(Move { from, to });
                            if !self.get_piece(&from).unwrap().can_snipe() {
                                break;
                            }
                            to += v;
                        }
                    }
                }
            }
        }
    }

    fn has_piece(&self, position: &Position) -> bool {
        self.get_piece(position).is_some()
    }

    fn add_castle_moves(&mut self) {
        let (king_square, kingside, queenside) = match self.player {
            Player::White => (
                CastleRights::WHITE_KING,
                CastleRights::WhiteKingside,
                CastleRights::WhiteQueenside,
            ),
            Player::Black => (
                CastleRights::BLACK_KING,
                CastleRights::BlackKingside,
                CastleRights::BlackQueenside,
            ),
        };

        if self.castle_rights[kingside as usize]
            && !(1..=2).any(|i| self.has_piece(&(king_square + Displacement::RIGHT * i)))
        {
            self.moves.insert(Move {
                from: king_square,
                to: king_square + Displacement::RIGHT * 2,
            });
        }

        if self.castle_rights[queenside as usize]
            && !(1..=3).any(|i| self.has_piece(&(king_square + Displacement::LEFT * i)))
        {
            self.moves.insert(Move {
                from: king_square,
                to: king_square + Displacement::LEFT * 2,
            });
        }
    }

    fn update_castling_rights(&mut self) {
        if self.get_piece(&CastleRights::WHITE_KINGSIDE_ROOK) != Some(Piece::Rook(Player::White)) {
            self.castle_rights[CastleRights::WhiteKingside as usize] = false;
        }
        if self.get_piece(&CastleRights::WHITE_QUEENSIDE_ROOK) != Some(Piece::Rook(Player::White)) {
            self.castle_rights[CastleRights::WhiteQueenside as usize] = false;
        }
        if self.get_piece(&CastleRights::BLACK_KINGSIDE_ROOK) != Some(Piece::Rook(Player::Black)) {
            self.castle_rights[CastleRights::BlackKingside as usize] = false;
        }
        if self.get_piece(&CastleRights::BLACK_QUEENSIDE_ROOK) != Some(Piece::Rook(Player::Black)) {
            self.castle_rights[CastleRights::BlackQueenside as usize] = false;
        }
        if self.get_piece(&CastleRights::WHITE_KING) != Some(Piece::King(Player::White)) {
            self.castle_rights[CastleRights::WhiteKingside as usize] = false;
            self.castle_rights[CastleRights::WhiteQueenside as usize] = false;
        }
        if self.get_piece(&CastleRights::BLACK_KING) != Some(Piece::King(Player::Black)) {
            self.castle_rights[CastleRights::BlackKingside as usize] = false;
            self.castle_rights[CastleRights::BlackQueenside as usize] = false;
        }
    }

    fn handle_castling_the_rook(&mut self, mv: &Move) {
        let (king, kingside_rook, queenside_rook) = match self.player {
            Player::White => (
                CastleRights::WHITE_KING,
                CastleRights::WHITE_KINGSIDE_ROOK,
                CastleRights::WHITE_QUEENSIDE_ROOK,
            ),
            Player::Black => (
                CastleRights::BLACK_KING,
                CastleRights::BLACK_KINGSIDE_ROOK,
                CastleRights::BLACK_QUEENSIDE_ROOK,
            ),
        };
        if mv.from == king {
            if mv.to == king + Displacement::RIGHT * 2 {
                let rook = self.take_piece(&kingside_rook);
                self.set_piece(&(kingside_rook + Displacement::LEFT * 2), rook);
            } else if mv.to == king + Displacement::LEFT * 2 {
                let rook = self.take_piece(&queenside_rook);
                self.set_piece(&(queenside_rook + Displacement::RIGHT * 3), rook);
            }
        }
    }

    fn was_double_move(&self, mv: &Move) -> bool {
        if let Some(Piece::Pawn(player)) = self.get_piece(&mv.to) {
            return match player {
                Player::White => mv.from.y == 1 && mv.to.y == 3,
                Player::Black => mv.from.y == 6 && mv.to.y == 4,
            };
        }
        false
    }

    fn update_en_passant(&mut self, mv: &Move) {
        self.en_passant_square = if self.was_double_move(mv) {
            Some(mv.from + Displacement::get_pawn_advance_vector(self.player))
        } else {
            None
        }
    }

    fn handle_capturing_en_passant(&mut self, to: &Position) {
        if Some(*to) == self.en_passant_square {
            self.set_piece(
                &(*to - Displacement::get_pawn_advance_vector(self.player)),
                None,
            );
        }
    }

    pub fn add_moves(&mut self) {
        self.moves.clear();
        for y in 0..8 {
            for x in 0..8 {
                self.add_moves_for_piece(Position { x, y })
            }
        }
        self.add_castle_moves();
    }
}

// Add unit tests at the bottom of each file. Each tests module should only have access to super (non integration)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_piece() {
        let mut board: Board = Board::new();
        let from = Position { x: 0, y: 1 };
        let to = Position { x: 0, y: 2 };
        let mv = Move { from, to };
        board.moves.insert(mv);
        board.move_piece(&mv).unwrap();
    }
}
