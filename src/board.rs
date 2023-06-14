use crate::game::ChessError;
use crate::pieces::{Piece, Player, Position};
use std::collections::HashSet;

const BOARD_SIZE: usize = 8;

type ChessResult<T> = Result<T, ChessError>;

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

        let mut moves = HashSet::new();

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

    pub fn is_in_bounds(position: Position) -> ChessResult<()> {
        if position.x > 7 || position.y > 7 {
            Err(ChessError::OutOfBounds)
        } else {
            Ok(())
        }
    }

    fn is_piece_some(&self, position: Position) -> ChessResult<()> {
        if self.get_piece(position)?.is_none() {
            Err(ChessError::NoPieceAtPosition)
        } else {
            Ok(())
        }
    }

    fn is_move_valid(&self, from: Position, to: Position) -> ChessResult<()> {
        //we need ending position to be inbound
        Self::is_in_bounds(to)?;

        //moving piece must be in bound and not `None`
        self.is_piece_some(from)?;
        
        if let Some(piece) = self._get_piece(from) {
            match piece {
                Piece::Pawn(..) => {}
                Piece::Knight(..) => {}
                Piece::Bishop(..) => {}
                Piece::Rook(..) => {}
                Piece::Queen(..) => {}
                Piece::King(..) => {}
            };
            Ok(())
        }
        else {
            Err(ChessError::NoPieceAtPosition)
        }
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult<()> {
        self.is_piece_some(from)?;
        self.is_move_valid(from, to)?;

        self.squares[to.x][to.y] = self.squares[from.x][from.y].take();

        Ok(())
    }

    pub fn get_piece(&self, position: Position) -> ChessResult<Option<Piece>> {
        Self::is_in_bounds(position)?;
        Ok(self._get_piece(position))
    }

    fn _get_piece(&self, position: Position) -> Option<Piece> {
        self.squares[position.x][position.y]
    }

    fn add_moves_in_direction(&mut self, start: Position, direction: Position) -> ChessResult<()> {
        let mut position = start + direction;
        while Self::is_in_bounds(position).is_ok() {
            if let Some(piece) = self.get_piece(position)? {
                // allow capturing an opponent's piece
                if piece.get_player() != self.player {
                    self.moves.insert((start, position));
                }
            } else {
                return Err(ChessError::NoPieceAtPosition)
            }
            self.moves.insert((start, position));
            position += direction;
        }
        Ok(())
    }
}
