use crate::game::ChessError;
use crate::pieces::{Piece, Player, Position};
use std::collections::HashSet;

const BOARD_SIZE: usize = 8;

pub struct Board {
    squares: [[Piece; BOARD_SIZE]; BOARD_SIZE],
    moves: HashSet<(Position, Position)>,
    player: Player,
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
        squares[0] = Self::get_back_rank(Player::White);
        squares[BOARD_SIZE - 1] = Self::get_back_rank(Player::Black);

        let mut moves = HashSet::new();

        let mut player = Player::White;

        Self {
            squares,
            moves,
            player,
        }
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

    pub fn is_in_bounds(position: Position) -> Result<(), ChessError> {
        if position.x > 7 || position.y > 7 {
            Err(ChessError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    fn is_valid_move(&self, from: Position, to: Position) -> Result<(), ChessError> {
        Self::is_in_bounds(from)?;
        Self::is_in_bounds(to)?;

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
        Self::is_in_bounds(from)?;
        Self::is_in_bounds(to)?;

        if self._get_piece(from).is_none() {
            return Err(ChessError::NoPieceAtPosition);
        }
        // Check if move is valid
        self.is_valid_move(from, to)?;

        // Perform the move
        self.squares[to.x][to.y] = self.squares[from.x][from.y].take();

        Ok(())
    }

    pub fn get_piece(&self, position: Position) -> Result<Piece, ChessError> {
        Self::is_in_bounds(position)?;
        Ok(self._get_piece(position))
    }

    fn _get_piece(&self, position: Position) -> Piece {
        self.squares[position.x][position.y]
    }

    fn add_moves_in_direction(&mut self, start: Position, direction: Position) {
        let mut position = start + direction;
        while Self::is_in_bounds(position).is_ok() {
            if let Some(player) = self._get_piece(position).get_player() {
                // allow capturing an opponent's piece
                if player != self.player {
                    self.moves.insert((start, position));
                }
                return;
            }
            self.moves.insert((start, position));
            position += direction;
        }
    }
}
