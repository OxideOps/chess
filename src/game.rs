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

#[derive(Clone, Default, PartialEq)]
pub enum GameStatus {
    #[default]
    Ongoing,
    Stalemate,
    Check,
    Checkmate,
}

#[derive(Clone, Default)]
pub struct Game {
    state: BoardState,
    valid_moves: HashSet<Move>,
    status: GameStatus,
}

impl Game {
    pub fn new() -> Self {
        let mut game = Self::default();
        game.add_moves();
        game
    }

    pub fn get_piece(&self, position: &Position) -> Option<Piece> {
        self.state.get_piece(position)
    }

    pub fn get_board_state(&self) -> &BoardState {
        &self.state
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult {
        if let Some(piece) = self.state.get_piece(&from) {
            let mv = Move::new(from, to);

            self.is_move_valid(&mv)?;
            self.state.move_piece(&mv);
            self.add_moves();
            self.remove_self_checks();
            self.update_status();

            println!("{} : {}", piece, mv);
        }
        Ok(())
    }

    fn update_status(&mut self) {
        let mut next_turn = self.clone();
        next_turn.state.player = !next_turn.state.player;
        next_turn.add_moves();
        if next_turn.has_check() {
            if self.valid_moves.is_empty() {
                self.status = GameStatus::Checkmate;
            } else {
                self.status = GameStatus::Check;
            }
        } else if self.valid_moves.is_empty() {
            self.status = GameStatus::Stalemate;
        } else {
            self.status = GameStatus::Ongoing;
        }
    }

    fn has_check(&self) -> bool {
        self.valid_moves
            .iter()
            .any(|m| self.get_piece(&m.to) == Some(Piece::King(!self.state.player)))
    }

    fn remove_self_checks(&mut self) {
        for mv in self.valid_moves.clone() {
            let mut next_turn = self.clone();
            next_turn.state.move_piece(&mv);
            next_turn.add_moves();
            if next_turn.has_check() {
                self.valid_moves.remove(&mv);
            }
        }
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
            if BoardState::is_in_bounds(&to).is_ok() && self.state.get_piece(&to).is_none() && self.can_double_move(&from) {
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
        self.add_castling_moves()
    }

    pub fn add_castling_moves(&mut self) {
        let (king_square, kingside, queenside) =
            CastlingRights::get_castling_info(self.state.player);

        if self.state.castling_rights.has_castling_right(kingside)
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

        if self.state.castling_rights.has_castling_right(queenside)
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

    pub fn has_piece(&self, position: &Position) -> bool {
        self.state.has_piece(position)
    }
}
