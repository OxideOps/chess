use crate::game::{ChessError, ChessResult};
use crate::moves::Move;
use crate::pieces::{Piece, Player, Position};
use std::collections::HashSet;

const BOARD_SIZE: usize = 8;

pub struct Board {
    squares: [[Option<Piece>; BOARD_SIZE]; BOARD_SIZE],
    moves: HashSet<(Position, Position)>,
    player: Player,
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

    pub fn get_piece(&self, position: Position) -> Option<Piece> {
        self.squares[position.y][position.x]
    }

    fn is_in_bounds(position: Position) -> ChessResult<()> {
        if position.x > 7 || position.y > 7 {
            Err(ChessError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    fn is_piece_some(&self, position: Position) -> ChessResult<()> {
        if let None = self.get_piece(position) {
            Err(ChessError::NoPieceAtPosition)
        } else {
            Ok(())
        }
    }

    fn is_move_valid(&self, from: Position, to: Position) -> ChessResult<()> {
        Self::is_in_bounds(from)?;
        Self::is_in_bounds(to)?;
        self.is_piece_some(from)?;
        if self.moves.contains(&(from, to)) {
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult<()> {
        self.is_move_valid(from, to)?;
        let mut piece = self.get_piece(from);

        if let Some(Piece::Pawn(player)) = piece {
            if (player == Player::White && to.y == 7) || (player == Player::Black && to.y == 0) {
                // TODO: always promote to queen for now, need to handle this eventually
                piece = Some(Piece::Queen(player));
            }
        }
        self.squares[to.y][to.x] = piece;
        self.squares[from.y][from.x] = None;
        Ok(())
    }

    fn pawn_can_double_move(&self, position: Position, player: Player) -> bool {
        let m = Move::get_pawn_advance_move(player);
        if let None = self.get_piece(position + m * 2) {
            return match self.get_piece(position).unwrap().get_player() {
                Player::White => position.y == 1,
                Player::Black => position.y == 6,
            };
        }
        false
    }

    fn add_pawn_advance_moves(&mut self, start: Position, player: Player) {
        let m = Move::get_pawn_advance_move(player);
        let new_position = start + m;
        if Self::is_in_bounds(new_position).is_ok() && self.get_piece(new_position).is_none() {
            self.moves.insert((start, new_position));
            if self.pawn_can_double_move(start, player) {
                self.moves.insert((start, new_position + m));
            }
        }
    }

    fn add_pawn_capture_moves(&mut self, start: Position, player: Player) {
        let capture_moves = match player {
            Player::White => Move::get_pawn_capture_moves_white(),
            Player::Black => Move::get_pawn_capture_moves_black(),
        };

        for &m in capture_moves {
            let new_position = start + m;
            if Self::is_in_bounds(new_position).is_ok() {
                if let Some(other_piece) = self.get_piece(new_position) {
                    if other_piece.get_player() != player {
                        self.moves.insert((start, new_position));
                    }
                }
            }
        }
    }

    fn add_moves_in_direction(&mut self, start: Position, piece: Piece) {
        match piece {
            Piece::Pawn(player) => {
                self.add_pawn_advance_moves(start, player);
                self.add_pawn_capture_moves(start, player);
            }
            _ => {
                for &m in piece.get_moves() {
                    let mut position = start + m;
                    while Self::is_in_bounds(position).is_ok() {
                        if let Some(piece) = self.get_piece(position) {
                            if piece.get_player() != self.player {
                                self.moves.insert((start, position));
                            }
                            break;
                        }
                        self.moves.insert((start, position));
                        if !self.get_piece(start).unwrap().can_snipe() {
                            break;
                        }
                        position += m;
                    }
                }
            }
        }
    }

    fn add_moves(&mut self) {
        for y in 0..8 {
            for x in 0..8 {
                if let Some(piece) = self.squares[y][x] {
                    self.add_moves_in_direction(Position { x, y }, piece);
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
        board
            .moves
            .insert((Position { x: 0, y: 1 }, Position { x: 0, y: 2 }));
        board
            .move_piece(Position { x: 0, y: 1 }, Position { x: 0, y: 2 })
            .unwrap();
    }
}
