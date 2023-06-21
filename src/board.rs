use crate::displacement::Displacement;
use crate::game::{ChessError, ChessResult};
use crate::moves::Move;
use crate::pieces::{Piece, Player, Position};
use std::collections::HashSet;

const BOARD_SIZE: usize = 8;

pub struct Board {
    squares: [[Option<Piece>; BOARD_SIZE]; BOARD_SIZE],
    moves: HashSet<Move>,
    pub player: Player,
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

    pub fn get_piece(&self, pos: &Position) -> Option<Piece> {
        self.squares[pos.y][pos.x]
    }

    fn is_in_bounds(pos: &Position) -> ChessResult<()> {
        if pos.x > 7 || pos.y > 7 {
            Err(ChessError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    fn is_piece_some(&self, pos: &Position) -> ChessResult<()> {
        if let None = self.get_piece(pos) {
            Err(ChessError::NoPieceAtPosition)
        } else {
            Ok(())
        }
    }

    fn is_move_valid(&self, m: &Move) -> ChessResult<()> {
        Self::is_in_bounds(&m.from)?;
        Self::is_in_bounds(&m.to)?;
        self.is_piece_some(&m.from)?;
        if self.moves.contains(m) {
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }

    pub fn move_piece(&mut self, m: &Move) -> ChessResult<()> {
        self.is_move_valid(&m)?;
        let mut piece = self.get_piece(&m.from);

        if let Some(Piece::Pawn(player)) = piece {
            if (player == Player::White && m.to.y == 7) || (player == Player::Black && m.to.y == 0)
            {
                // TODO: always promote to queen for now, need to handle this eventually
                piece = Some(Piece::Queen(player));
            }
        }
        self.squares[m.to.y][m.to.x] = piece;
        self.squares[m.from.y][m.from.x] = None;
        Ok(())
    }

    pub fn next_turn(&mut self) {
        self.player = match self.player {
            Player::White => Player::Black,
            Player::Black => Player::White,
        };
        self.add_moves();
    }

    fn add_pawn_advance_moves(&mut self, from: Position, player: Player) {
        let v = Displacement::get_pawn_advance_vector(player);
        let mut to = from + v;
        if Self::is_in_bounds(&to).is_ok() && self.get_piece(&to).is_none() {
            self.moves.insert(Move { from, to });
            //check for double move
            to += v;
            if self.get_piece(&to).is_none() {
                let can_double_move = match self.get_piece(&from).unwrap().get_player() {
                    Player::White => from.y == 1,
                    Player::Black => from.y == 6,
                };
                if can_double_move {
                    self.moves.insert(Move { from, to });
                }
            }
        }
    }

    fn add_pawn_capture_moves(&mut self, from: Position, player: Player) {
        let capture_vectors = match player {
            Player::White => Displacement::get_white_pawn_capture_vectors(),
            Player::Black => Displacement::get_black_pawn_capture_vectors(),
        };

        for &v in capture_vectors {
            let to = from + v;
            if Self::is_in_bounds(&to).is_ok() {
                if let Some(other_piece) = self.get_piece(&to) {
                    if other_piece.get_player() != player {
                        self.moves.insert(Move { from, to });
                    }
                }
            }
        }
    }

    fn add_moves_in_direction(&mut self, from: Position, piece: Piece) {
        match piece {
            Piece::Pawn(player) => {
                self.add_pawn_advance_moves(from, player);
                self.add_pawn_capture_moves(from, player);
            }
            _ => {
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

    pub fn add_moves(&mut self) {
        self.moves.clear();
        for y in 0..8 {
            for x in 0..8 {
                if let Some(piece) = self.squares[y][x] {
                    if piece.get_player() == self.player {
                        self.add_moves_in_direction(Position { x, y }, piece);
                    }
                }
            }
        }
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
        let m = Move { from, to };
        board.moves.insert(m);
        board.move_piece(&m).unwrap();
    }
}
