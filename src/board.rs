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

    fn is_valid_move(&self, from: Position, to: Position) -> Result<(), ChessError> {
        self.is_in_bounds(from)?;
        self.is_in_bounds(to)?;

        let piece = &self.squares[from.x][from.y];

        match *piece {
            Piece::Pawn(..) => {}
            Piece::Knight(..) => {}
            Piece::Bishop(..) => {}
            Piece::Rook(..) => {}
            Piece::Queen(..) => {}
            Piece::King(..) => {}
            Piece::None => return Err(ChessError::InvalidMove),
        };
        Ok(())
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> Result<(), ChessError> {
        // Check if 'from' and 'to' positions are in bounds
             self.is_in_bounds(from)?;
        self.is_in_bounds(to)?;

        if self.squares[from.x][from.y].is_none() {
            return Err(ChessError::NoPieceAtPosition);
        }
        // Check if move is valid
        self.is_valid_move(from, to)?;

        // Perform the move
        self.squares[to.x][to.y] = self.squares[from.x][from.y].take();

        Ok(())
    }
}
