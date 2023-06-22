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

    pub fn get_piece(&self, pos: &Position) -> &Option<Piece> {
        &self.squares[pos.y][pos.x]
    }

    fn get_piece_mut(&mut self, pos: &Position) -> &mut Option<Piece> {
        &mut self.squares[pos.y][pos.x]
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

    fn is_move_valid(&self, mv: &Move) -> ChessResult<()> {
        Self::is_in_bounds(&mv.from)?;
        Self::is_in_bounds(&mv.to)?;
        self.is_piece_some(&mv.from)?;
        if self.moves.contains(mv) {
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }

    pub fn move_piece(&mut self, mv: &Move) -> ChessResult<()> {
        self.is_move_valid(mv)?;

        if let Some(mut piece) = self.get_piece_mut(&mv.from).take() {
            if let Piece::Pawn(player) = piece {
                if (player == Player::White && mv.to.y == BOARD_SIZE - 1)
                    || (player == Player::Black && mv.to.y == 0)
                {
                    piece = Piece::Queen(player)
                }
            }
            self.squares[mv.to.y][mv.to.x] = Some(piece);
        }
        Ok(())
    }

    pub fn next_turn(&mut self) {
        self.player = [Player::Black, Player::White][self.player as usize];
        self.add_moves();
    }

    fn add_pawn_advance_moves(&mut self, from: Position, player: &Player) {
        let v = Displacement::get_pawn_advance_vector(player);
        let mut to = from + v;
        if Self::is_in_bounds(&to).is_ok() && self.get_piece(&to).is_none() {
            self.moves.insert(Move { from, to });
            //check for double move
            to += v;
            if self.get_piece(&to).is_none() {
                let player = self.get_piece(&from).unwrap().get_player();
                if [from.y == 1, from.y == 6][player as usize] {
                    self.moves.insert(Move { from, to });
                }
            }
        }
    }

    fn add_pawn_capture_moves(&mut self, from: Position, player: &Player) {
        let capture_vectors = match player {
            Player::White => Displacement::get_white_pawn_capture_vectors(),
            Player::Black => Displacement::get_black_pawn_capture_vectors(),
        };

        for &v in capture_vectors {
            let to = from + v;
            if Self::is_in_bounds(&to).is_ok() {
                if let Some(piece) = self.get_piece(&to) {
                    if &piece.get_player() != player {
                        self.moves.insert(Move { from, to });
                    }
                }
            }
        }
    }

    fn add_moves_for_piece(&mut self, from: Position) {
        if let &Some(piece) = self.get_piece(&from) {
            if piece.get_player() == self.player {
                if let Piece::Pawn(player) = piece {
                    self.add_pawn_advance_moves(from, &player);
                    self.add_pawn_capture_moves(from, &player);
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

    pub fn add_moves(&mut self) {
        self.moves.clear();
        for y in 0..8 {
            for x in 0..8 {
                self.add_moves_for_piece(Position { x, y })
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
        let mv = Move { from, to };
        board.moves.insert(mv);
        board.move_piece(&mv).unwrap();
    }
}
