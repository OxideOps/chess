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

    pub fn move_piece(&mut self, from: Position, to: Position) -> Result<(), ChessError> {
        self.is_in_bounds(to)?;

        let piece = &mut self.squares[from.x][from.y];

        if piece.is_none() {
            return Err(ChessError::NoPieceAtPosition);
        }

        match *piece {
            Piece::Pawn(..) => {
                //Err(ChessError::PawnError::SomeError)
                return Ok(());
            }
            Piece::Knight(..) => return Ok(()),
            Piece::Bishop(..) => return Ok(()),
            Piece::Rook(..) => {
                //Err(ChessError::RookError::SomeOtherError)
                return Ok(());
            }
            Piece::Queen(..) => return Ok(()),
            Piece::King(..) => return Ok(()),
            Piece::None => return Ok(()),
        };
        Ok(())
    }
}
