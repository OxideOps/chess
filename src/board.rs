use crate::game::ChessError;
use crate::pieces::{Piece, Player, Position};

const BOARD_SIZE: usize = 8;

pub struct Board {
    squares: [[Piece; 8]; 8],
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

        for i in 0..BOARD_SIZE {
            squares[0] = Board::get_back_rank(i, Player::White);
            squares[BOARD_SIZE - 1] = Board::get_back_rank(i, Player::Black);
        }

        Self { squares }
    }

    fn get_back_rank(file: usize, player: Player) -> [Piece; 8] {
        return [
            Piece::Rook(player),
            Piece::Knight(player),
            Piece::Bishop(player),
            Piece::Queen(player),
            Piece::King(player),
            Piece::Bishop(player),
            Piece::Knight(player),
            Piece::Rook(player),
        ];
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

    pub fn take_piece_at(&mut self, position: Position) -> Result<Piece, ChessError> {
        self.is_in_bounds(position)?;
        Ok(self.squares[position.x][position.y].take())
    }

    pub fn set_piece_at(&mut self, position: Position, piece: Piece) -> Result<(), ChessError> {
        self.is_in_bounds(position)?;
        self.squares[position.x][position.y] = piece;
        Ok(())
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> Result<(), ChessError> {
        if self.borrow_piece_at(from)?.is_none() {
            return Err(ChessError::NoPieceAtPosition);
        }

        let piece = self.take_piece_at(from)?;

        if piece.can_move(self, from, to)? {
            self.set_piece_at(to, piece)?;
        }
        Ok(())
    }
}
