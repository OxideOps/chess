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

        let moves = HashSet::new();

        let mut player = Player::White;

        Self {
            squares,
            moves,
            player,
        }
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

    fn get_piece(&self, position: Position) -> Option<Piece> {
        self.squares[position.x][position.y]
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
        self.squares[to.x][to.y] = self.get_piece(from).take();

        Ok(())
    }

    fn get_moves(piece: Piece) -> &'static [Move] {
        // not exactly sure how to handle pawns yet
        match piece {
            Piece::Rook(..) => Move::get_rook_moves(),
            Piece::Bishop(..) => Move::get_bishop_moves(),
            Piece::Knight(..) => Move::get_knight_moves(),
            Piece::Queen(..) => Move::get_queen_moves(),
            Piece::King(..) => Move::get_king_moves(),
            _ => Default::default(),
        }
    }

    fn add_moves_in_direction(&mut self, start: Position, m: Move) {
        let mut position = start + m;
        while Self::is_in_bounds(position).is_ok() {
            if let Some(piece) = self.get_piece(position) {
                // allow capturing an opponent's piece
                if piece.get_player() != self.player {
                    self.moves.insert((start, position));
                }
                return;
            }
            self.moves.insert((start, position));
            position += m;
        }
    }
}
