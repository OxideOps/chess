
use crate::board::Board;
use crate::game::ChessError;

pub trait Piece {
    fn color(&self) -> Color;
    fn has_valid_move(&self) -> bool;
    fn update_valid_moves(&self) -> usize;

    fn perform_move(&self, board: &mut Board, from: Position, to: Position) -> Result<(), ChessError>;
    // Add other common methods here
}

impl Piece for Pawn {
    fn color(&self) -> Color {
        self.color
    }
    
    fn perform_move(&self, board: &mut Board, from: Position, to: Position) -> Result<(), ChessError> {
        if !self.has_valid_move() {
            return Err(ChessError::NoValidMoves)
        }

        if piece.can_move(self, from, to) {
            board.squares[to.x][to.y] = self.squares[from.x][from.y].take();
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }

    fn can_move(&self, board: &Board, from: Position, to: Position) -> bool {
        match self.color {
            Color::White => {
                self.can_advance_single_square(board, from, to) ||
                self.can_advance_double_square(board, from, to, 1, 3) ||
                self.can_capture(board, from, to, Color::Black)
            }
            Color::Black => {
                self.can_advance_single_square(board, from, to) ||
                self.can_advance_double_square(board, from, to, 6, 4) ||
                self.can_capture(board, from, to, Color::White)
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct Pawn {
    color: Color,
    valid_moves: usize,
}

impl Pawn {
    fn can_advance_single_square(&self, board: &Board, from: Position, to: Position) -> bool {
        let direction = match self.color {
            Color::White => 1,
            Color::Black => -1,
        };
        let is_correct_direction = to.y as isize == from.y as isize + direction;
        let is_same_file = from.x == to.x;
        let destination_is_empty = if let Ok(piece_opt) = board.get_piece(to) {
            piece_opt.is_none()
        } else {
            false
        };
        is_correct_direction && is_same_file && destination_is_empty
    }
    
    fn can_advance_double_square(&self, board: &Board, from: Position, to: Position, start_row: usize, end_row: usize) -> bool {
        let is_starting_position = from.y == start_row;
        let is_correct_end_position = to.y == end_row;
        let is_same_file = from.x == to.x;
        let path_is_clear = if let Ok(piece_opt) = board.get_piece(Position { x: to.x, y: (start_row + end_row) / 2 }) {
            piece_opt.is_none()
        } else {
            false
        };
        let destination_is_empty = if let Ok(piece_opt) = board.get_piece(to) {
            piece_opt.is_none()
        } else {
            false
        };
        is_starting_position && is_correct_end_position && is_same_file && path_is_clear && destination_is_empty
    }
    
    fn can_capture(&self, board: &Board, from: Position, to: Position, opponent_color: Color) -> bool {
        let direction = match self.color {
            Color::White => 1,
            Color::Black => -1,
        };
        let is_correct_direction = to.y as isize == from.y as isize + direction;
        let is_diagonal_move = to.x == from.x + 1 || to.x == from.x - 1;
        let is_opponent_piece = if let Ok(Some(piece)) = board.get_piece(to) {
            piece.color() == opponent_color
        } else {
            false
        };
        is_correct_direction && is_diagonal_move && is_opponent_piece
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Color {
    White,
    Black,
}

#[derive(Clone, Copy)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}
