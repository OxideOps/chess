use crate::game::{ChessError, ChessResult};
use crate::pieces::{Move, Piece, Player, Position};
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
    
        if let Some(piece) = self.get_piece(from) {
            let moves = Self::get_moves(piece);
            for m in moves {
                let new_pos = Position { x: from.x + m.dx as usize, y: from.y + m.dy as usize};
                if new_pos == to {
                    return Ok(());  // Move is valid
                }
            }
        }
        Err(ChessError::InvalidMove)
    }
    

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult<()> {
        self.is_move_valid(from, to)?;
        self.squares[to.x][to.y] = self.get_piece(from).take();

        Ok(())
    }

    fn get_rook_directions() -> &'static [Move] {
        &[Move { dx: 0, dy: 1 }]
    }

    fn get_bishop_directions() -> &'static [Move] {
        &[
            Move { dx: 1, dy: 0 }, 
            Move { dx: -1, dy: 0 }, 
            Move { dx: 0, dy: 1 }, 
            Move { dx: 0, dy: -1 }
        ]
    }
    /* 
    fn get_knight_directions() -> Vec<Direction> {
        vec![
            Direction { x: 1, y: 2 },
            Direction { x: 1, y: -2 },
            Direction { x: -1, y: 2 },
            Direction { x: -1, y: -2 },
            Direction { x: 2, y: 1 },
            Direction { x: 2, y: -1 },
            Direction { x: -2, y: 1 },
            Direction { x: -2, y: -1 },
        ]
    }
    
    
    fn get_queen_directions() -> Vec<Direction> {
        [Self::get_rook_directions(), Self::get_bishop_directions()].concat()
    }

    fn get_king_directions() -> Vec<Direction> {
        Self::get_queen_directions()
    }
    */
    fn get_moves(piece: Piece) -> &'static [Move] {
        // not exactly sure how to handle pawns yet
        match piece {
            Piece::Rook(..) => Self::get_rook_directions(),
            Piece::Bishop(..) => Self::get_bishop_directions(),
            //Piece::Knight(..) => Self::get_knight_directions(),
            //Piece::Queen(..) => Self::get_queen_directions(),
            //Piece::King(..) => Self::get_king_directions(),
            _ => Default::default()
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
            }
            self.moves.insert((start, position));
            position += m;
        }
    }
}
