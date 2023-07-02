use std::collections::HashSet;

use crate::board::BoardState;
use crate::castling_rights::CastlingRights;
use crate::displacement::Displacement;
use crate::moves::Move;
use crate::pieces::{Piece, Player, Position};

pub type ChessResult = Result<(), ChessError>;
#[derive(Debug)]
pub enum ChessError {
    OutOfBounds,
    NoPieceAtPosition,
    InvalidMove,
    OwnPieceInDestination,
    PlayerInCheck,
    Checkmate,
    Stalemate,
    InvalidPromotion,
    NotPlayersTurn,
    EmptyPieceMove,
}

#[derive(Default, PartialEq)]
pub enum GameStatus {
    #[default]
    Ongoing,
    Stalemate,
    Check,
    Checkmate,
}

pub struct Game {
    state: BoardState,
    valid_moves: HashSet<Move>,
    status: GameStatus,
}

impl Default for Game {
    fn default() -> Self {
        let (state, valid_moves, status) = Default::default();
        let mut game = Game {
            state,
            valid_moves,
            status,
        };
        game.add_moves();
        game
    }
}

impl Game {
    pub fn new() -> Self {
        let mut game = Game::default();
        game.add_moves();
        game
    }

    pub fn get_piece(&self, position: &Position) -> Option<Piece> {
        self.state.get_piece(position)
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult {
        if let Some(piece) = self.state.get_piece(&from) {
            let mv = Move::new(from, to);

            self.is_move_valid(&mv)?;
            self.state.move_piece(&mv);
            self.add_moves();

            println!("{} : {}", piece, mv);
        }
        Ok(())
    }

    fn is_move_valid(&self, mv: &Move) -> ChessResult {
        BoardState::is_in_bounds(&mv.from)?;
        BoardState::is_in_bounds(&mv.to)?;
        self.state.is_piece_some(&mv.from)?;

        if self.valid_moves.contains(mv) {
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }

    fn add_pawn_advance_moves(&mut self, from: Position) {
        let v = Displacement::get_pawn_advance_vector(self.state.player);
        let mut to = from + v;
        if BoardState::is_in_bounds(&to).is_ok() && self.state.get_piece(&to).is_none() {
            self.valid_moves.insert(Move { from, to });
            to += v;
            if self.state.get_piece(&to).is_none() && self.can_double_move(&from) {
                self.valid_moves.insert(Move { from, to });
            }
        }
    }

    fn can_double_move(&self, from: &Position) -> bool {
        if let Piece::Pawn(player) = self.state.get_piece(from).unwrap() {
            return match player {
                Player::White => from.y == 1,
                Player::Black => from.y == 6,
            };
        }
        false
    }

    fn add_pawn_capture_moves(&mut self, from: Position) {
        for &v in Displacement::get_pawn_capture_vectors(self.state.player) {
            let to = from + v;
            if BoardState::is_in_bounds(&to).is_ok() {
                if let Some(piece) = self.state.get_piece(&to) {
                    if piece.get_player() != self.state.player {
                        self.valid_moves.insert(Move::new(from, to));
                    }
                }
                if Some(to) == self.state.en_passant_position {
                    self.valid_moves.insert(Move::new(from, to));
                }
            }
        }
    }

    fn add_moves_for_piece(&mut self, from: Position) {
        if let Some(piece) = self.state.get_piece(&from) {
            if piece.get_player() == self.state.player {
                if piece.is_pawn() {
                    self.add_pawn_advance_moves(from);
                    self.add_pawn_capture_moves(from);
                } else {
                    for &v in piece.get_vectors() {
                        let mut to = from + v;
                        while BoardState::is_in_bounds(&to).is_ok() {
                            if let Some(piece) = self.state.get_piece(&to) {
                                if piece.get_player() != self.state.player {
                                    self.valid_moves.insert(Move { from, to });
                                }
                                break;
                            }
                            self.valid_moves.insert(Move { from, to });
                            if !self.state.get_piece(&from).unwrap().can_snipe() {
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
        self.valid_moves.clear();
        for y in 0..8 {
            for x in 0..8 {
                self.add_moves_for_piece(Position::new(x, y))
            }
        }
        self.add_castle_moves();
    }

    fn add_castle_moves(&mut self) {
        let (king_square, kingside, queenside) =
            CastlingRights::get_castling_info(self.state.player);

        if self.state.castle_rights[kingside as usize]
            && !(1..=2).any(|i| {
                self.state
                    .has_piece(&(king_square + Displacement::RIGHT * i))
            })
        {
            self.valid_moves.insert(Move {
                from: king_square,
                to: king_square + Displacement::RIGHT * 2,
            });
        }

        if self.state.castle_rights[queenside as usize]
            && !(1..=3).any(|i| {
                self.state
                    .has_piece(&(king_square + Displacement::LEFT * i))
            })
        {
            self.valid_moves.insert(Move {
                from: king_square,
                to: king_square + Displacement::LEFT * 2,
            });
        }
    }
}
