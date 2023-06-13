use crate::game::ChessError;
use crate::pieces::{Piece, Player, Position};

const BOARD_SIZE: usize = 8;

pub struct Board {
    squares: [[Piece; BOARD_SIZE]; BOARD_SIZE],
}

impl Board {
    pub fn new() -> Self {
        let mut squares = [[Piece::None; BOARD_SIZE]; BOARD_SIZE];

        // Initialize white pawns
        for i in 0..8 {
            squares[1][i] = Piece::Pawn(Player::White);
        }

        // Initialize black pawns
        for i in 0..8 {
            squares[6][i] = Piece::Pawn(Player::Black);
        }

        // Initialize the other white and black pieces
        squares[0] = Board::get_back_rank(Player::White);
        squares[BOARD_SIZE - 1] = Board::get_back_rank(Player::Black);

        Self { squares }
    }

    fn get_back_rank(player: Player) -> [Piece; 8] {
        [
            Piece::Rook(player),
            Piece::Knight(player),
            Piece::Bishop(player),
            Piece::Queen(player),
            Piece::King(player),
            Piece::Bishop(player),
            Piece::Knight(player),
            Piece::Rook(player),
        ]
    }

    pub fn is_in_bounds(&self, position: Position) -> Result<(), ChessError> {
        if position.x > 7 || position.y > 7 {
            Err(ChessError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    pub fn borrow_piece_at(&self, position: Position) -> Result<&Piece, ChessError> {
        self.is_in_bounds(position)?;
        Ok(&self.squares[position.x][position.y])
    }

    pub fn move_piece_to(&mut self,  mut piece: Piece, position: Position) -> Result<(), ChessError> {
        self.is_in_bounds(position)?;
        self.squares[position.x][position.y] = piece.take();
        Ok(())
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> Result<(), ChessError> {
        self.is_in_bounds(to)?;

        let piece = self.borrow_piece_at(from)?;

        if piece.is_none() {
            return Err(ChessError::NoPieceAtPosition);
        }

        if piece.can_move(&self, to, from)? {
            self.move_piece_to(*piece, to)?;
            self.squares[from.x][from.y] = Piece::None;
        }

        Ok(())
    }
}
