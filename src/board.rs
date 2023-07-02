use crate::displacement::Displacement;
use crate::game::{ChessError, ChessResult, CastlingRights};
use crate::moves::Move;
use crate::pieces::{Piece, Player, Position};
use std::collections::HashSet;

const BOARD_SIZE: usize = 8;

type Square = Option<Piece>;

#[derive(PartialEq)]
pub struct Board([[Square; BOARD_SIZE]; BOARD_SIZE]);

impl Default for Board {
    fn default() -> Self {
        let mut squares = [[None; BOARD_SIZE]; BOARD_SIZE];

        // Initialize white pawns
        for i in 0..8 {
            squares[1][i] = Some(Piece::Pawn(Player::White));
            squares[6][i] = Some(Piece::Pawn(Player::Black));
        }

        // Initialize the other white and black pieces
        squares[0] = Self::get_back_rank(Player::White);
        squares[BOARD_SIZE - 1] = Self::get_back_rank(Player::Black);

        Self(squares)
    }
}

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_back_rank(player: Player) -> [Square; 8] {
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

    pub fn get_piece(&self, at: &Position) -> Square {
        self.0[at.y][at.x]
    }

    pub fn set_piece(&mut self, at: &Position, sq: Square) {
        self.0[at.y][at.x] = sq;
    }

    pub fn take_piece(&mut self, from: &Position) -> Square {
        self.0[from.y][from.x].take()
    }
}

#[derive(PartialEq)]
/// A struct encapsulating the state for the `Board`.
pub struct BoardState {
    pub player: Player,
    pub board: Board,
    valid_moves: HashSet<Move>,
    castle_rights: [bool; 4],
    en_passant_position: Option<Position>,
}

impl Default for BoardState {
    fn default() -> Self {
        let mut state = Self {
            board: Board::new(),
            valid_moves: HashSet::new(),
            player: Player::White,
            castle_rights: [true, true, true, true],
            en_passant_position: None,
        };
        state.add_moves();
        state
    }
}

impl BoardState {
    fn is_in_bounds(at: &Position) -> ChessResult {
        if at.x > 7 || at.y > 7 {
            Err(ChessError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    fn is_piece_some(&self, at: &Position) -> ChessResult {
        if !self.has_piece(at) {
            return Err(ChessError::NoPieceAtPosition);
        }
        Ok(())
    }

    fn is_move_valid(&self, mv: &Move) -> ChessResult {
        Self::is_in_bounds(&mv.from)?;
        Self::is_in_bounds(&mv.to)?;
        self.is_piece_some(&mv.from)?;

        if self.valid_moves.contains(mv) {
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }

    fn can_promote_piece(&self, at: &Position) -> bool {
        (self.player == Player::White && at.y == 7) || (self.player == Player::Black && at.y == 0)
    }

    /// Performs a `Move` if valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use chess::{board::BoardState, pieces::{Player, Piece, Position}, moves::Move};
    ///
    /// let mut state: BoardState = BoardState::default();
    /// let from = Position { x: 0, y: 1 };
    /// let to = Position { x: 0, y: 2 };
    /// let mv = Move { from, to };
    /// state.move_piece(&mv).unwrap();
    ///
    /// assert_eq!(state.board.get_piece(&from), None);
    /// assert_eq!(state.board.get_piece(&to), Some(Piece::Pawn(Player::White)));
    /// ```
    pub fn move_piece(&mut self, mv: &Move) -> ChessResult {
        self.is_move_valid(mv)?;

        let mut piece = self.board.take_piece(&mv.from).unwrap();
        if piece.is_pawn() && self.can_promote_piece(&mv.to) {
            piece = Piece::Queen(self.player)
        }
        self.board.0[mv.to.y][mv.to.x] = Some(piece);
        self.handle_castling_the_rook(mv);
        self.handle_capturing_en_passant(&mv.to);
        self.update_castling_rights();
        self.update_en_passant(mv);
        Ok(())
    }

    pub fn next_turn(&mut self) {
        self.player = !self.player;
        self.add_moves();
    }

    fn can_double_move(&self, from: &Position) -> bool {
        if let Piece::Pawn(player) = self.board.get_piece(from).unwrap() {
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
        if Self::is_in_bounds(&to).is_ok() && self.board.get_piece(&to).is_none() {
            self.valid_moves.insert(Move { from, to });
            to += v;
            if self.board.get_piece(&to).is_none() && self.can_double_move(&from) {
                self.valid_moves.insert(Move { from, to });
            }
        }
    }

    fn add_pawn_capture_moves(&mut self, from: Position) {
        for &v in Displacement::get_pawn_capture_vectors(self.player) {
            let to = from + v;
            if Self::is_in_bounds(&to).is_ok() {
                if let Some(piece) = self.board.get_piece(&to) {
                    if piece.get_player() != self.player {
                        self.valid_moves.insert(Move { from, to });
                    }
                }
                if Some(to) == self.en_passant_position {
                    self.valid_moves.insert(Move { from, to });
                }
            }
        }
    }

    fn add_moves_for_piece(&mut self, from: Position) {
        if let Some(piece) = self.board.get_piece(&from) {
            if piece.get_player() == self.player {
                if piece.is_pawn() {
                    self.add_pawn_advance_moves(from);
                    self.add_pawn_capture_moves(from);
                } else {
                    for &v in piece.get_vectors() {
                        let mut to = from + v;
                        while Self::is_in_bounds(&to).is_ok() {
                            if let Some(piece) = self.board.get_piece(&to) {
                                if piece.get_player() != self.player {
                                    self.valid_moves.insert(Move { from, to });
                                }
                                break;
                            }
                            self.valid_moves.insert(Move { from, to });
                            if !self.board.get_piece(&from).unwrap().can_snipe() {
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
        self.board.get_piece(position).is_some()
    }

    fn add_castle_moves(&mut self) {
        let (king_square, kingside, queenside) = match self.player {
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
        };

        if self.castle_rights[kingside as usize]
            && !(1..=2).any(|i| self.has_piece(&(king_square + Displacement::RIGHT * i)))
        {
            self.valid_moves.insert(Move {
                from: king_square,
                to: king_square + Displacement::RIGHT * 2,
            });
        }

        if self.castle_rights[queenside as usize]
            && !(1..=3).any(|i| self.has_piece(&(king_square + Displacement::LEFT * i)))
        {
            self.valid_moves.insert(Move {
                from: king_square,
                to: king_square + Displacement::LEFT * 2,
            });
        }
    }

    fn update_castling_rights(&mut self) {
        if self.board.get_piece(&CastlingRights::WHITE_KINGSIDE_ROOK)
            != Some(Piece::Rook(Player::White))
        {
            self.castle_rights[CastlingRights::WhiteKingside as usize] = false;
        }
        if self.board.get_piece(&CastlingRights::WHITE_QUEENSIDE_ROOK)
            != Some(Piece::Rook(Player::White))
        {
            self.castle_rights[CastlingRights::WhiteQueenside as usize] = false;
        }
        if self.board.get_piece(&CastlingRights::BLACK_KINGSIDE_ROOK)
            != Some(Piece::Rook(Player::Black))
        {
            self.castle_rights[CastlingRights::BlackKingside as usize] = false;
        }
        if self.board.get_piece(&CastlingRights::BLACK_QUEENSIDE_ROOK)
            != Some(Piece::Rook(Player::Black))
        {
            self.castle_rights[CastlingRights::BlackQueenside as usize] = false;
        }
        if self.board.get_piece(&CastlingRights::WHITE_KING) != Some(Piece::King(Player::White)) {
            self.castle_rights[CastlingRights::WhiteKingside as usize] = false;
            self.castle_rights[CastlingRights::WhiteQueenside as usize] = false;
        }
        if self.board.get_piece(&CastlingRights::BLACK_KING) != Some(Piece::King(Player::Black)) {
            self.castle_rights[CastlingRights::BlackKingside as usize] = false;
            self.castle_rights[CastlingRights::BlackQueenside as usize] = false;
        }
    }

    fn handle_castling_the_rook(&mut self, mv: &Move) {
        let (king, kingside_rook, queenside_rook) = match self.player {
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
        };
        if mv.from == king {
            if mv.to == king + Displacement::RIGHT * 2 {
                let rook = self.board.take_piece(&kingside_rook);
                self.board
                    .set_piece(&(kingside_rook + Displacement::LEFT * 2), rook);
            } else if mv.to == king + Displacement::LEFT * 2 {
                let rook = self.board.take_piece(&queenside_rook);
                self.board
                    .set_piece(&(queenside_rook + Displacement::RIGHT * 3), rook);
            }
        }
    }

    fn was_double_move(&self, mv: &Move) -> bool {
        if let Some(Piece::Pawn(player)) = self.board.get_piece(&mv.to) {
            return match player {
                Player::White => mv.from.y == 1 && mv.to.y == 3,
                Player::Black => mv.from.y == 6 && mv.to.y == 4,
            };
        }
        false
    }

    fn update_en_passant(&mut self, mv: &Move) {
        self.en_passant_position = if self.was_double_move(mv) {
            Some(mv.from + Displacement::get_pawn_advance_vector(self.player))
        } else {
            None
        }
    }

    fn handle_capturing_en_passant(&mut self, to: &Position) {
        if Some(*to) == self.en_passant_position {
            self.board.set_piece(
                &(*to - Displacement::get_pawn_advance_vector(self.player)),
                None,
            );
        }
    }

    pub fn add_moves(&mut self) {
        self.valid_moves.clear();
        for y in 0..8 {
            for x in 0..8 {
                self.add_moves_for_piece(Position { x, y })
            }
        }
        self.add_castle_moves();
    }
}
