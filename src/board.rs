use crate::castling_rights::CastlingRights;
use crate::displacement::Displacement;
use crate::game::{ChessError, ChessResult};
use crate::moves::Move;
use crate::pieces::{Color, Piece, Position};

const BOARD_SIZE: usize = 8;

type Square = Option<Piece>;

#[derive(Clone, PartialEq)]
pub struct Board([[Square; BOARD_SIZE]; BOARD_SIZE]);

impl Default for Board {
    fn default() -> Self {
        let mut squares = [[None; BOARD_SIZE]; BOARD_SIZE];

        // Initialize white pawns
        for i in 0..8 {
            squares[1][i] = Some(Piece::Pawn(Color::White));
            squares[6][i] = Some(Piece::Pawn(Color::Black));
        }

        // Initialize the other white and black pieces
        squares[0] = Self::get_back_rank(Color::White);
        squares[BOARD_SIZE - 1] = Self::get_back_rank(Color::Black);

        Self(squares)
    }
}

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_back_rank(player: Color) -> [Square; 8] {
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

#[derive(Clone, Default)]
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

    fn can_promote_piece(&self, at: &Position) -> bool {
        (self.player == Color::White && at.y == 7) || (self.player == Color::Black && at.y == 0)
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
        self.castling_rights
            .handle_castling_the_rook(mv, &mut self.board, self.player);
        self.castling_rights.update_castling_rights(&self.board);
        self.handle_capturing_en_passant(&mv.to);
        self.update_en_passant(mv);
        self.player = !self.player;
    }

    pub fn has_piece(&self, position: &Position) -> bool {
        self.board.get_piece(position).is_some()
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
}
