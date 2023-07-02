use crate::castling_rights::CastlingRights;
use crate::displacement::Displacement;
use crate::game::{ChessError, ChessResult};
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
    pub castle_rights: [bool; 4],
    pub en_passant_position: Option<Position>,
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            board: Board::new(),
            player: Player::White,
            castle_rights: [true, true, true, true],
            en_passant_position: None,
        }
    }
}

impl BoardState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_piece(&self, at: &Position) -> Square {
        self.board.get_piece(at)
    }

    fn can_promote_piece(&self, at: &Position) -> bool {
        (self.player == Player::White && at.y == 7) || (self.player == Player::Black && at.y == 0)
    }

    pub fn move_piece(&mut self, mv: &Move) {
        let mut piece = self.board.take_piece(&mv.from).unwrap();
        if piece.is_pawn() && self.can_promote_piece(&mv.to) {
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
        self.handle_castling_the_rook(mv);
        self.handle_capturing_en_passant(&mv.to);
        self.update_castling_rights();
        self.update_en_passant(mv);
        self.player = !self.player;
    }

    pub fn has_piece(&self, position: &Position) -> bool {
        self.board.get_piece(position).is_some()
    }

    fn add_castle_moves(&mut self) {
        let (king_square, kingside, queenside) = CastlingRights::get_castling_info(self.player);

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
        for &(position, piece, rights) in CastlingRights::rook_positions().as_ref() {
            if self.board.get_piece(&position) != Some(piece) {
                self.castle_rights[rights as usize] = false;
            }
        }

        for &(position, piece, kingside_rights, queenside_rights) in
            CastlingRights::king_positions().as_ref()
        {
            if self.board.get_piece(&position) != Some(piece) {
                self.castle_rights[kingside_rights as usize] = false;
                self.castle_rights[queenside_rights as usize] = false;
            }
        }
    }

    fn handle_castling_the_rook(&mut self, mv: &Move) {
        let (king, kingside_rook, queenside_rook) =
            CastlingRights::get_castling_positions(self.player);

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
}
