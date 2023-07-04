use crate::board::{Board, BoardState};
use crate::castling_rights::CastlingRights;
use crate::displacement::Displacement;
use crate::moves::Move;
use crate::pieces::{Piece, Player, Position};

use std::collections::HashSet;

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

#[derive(Clone)]
pub struct History {
    history: Vec<(BoardState, Move)>,
    current_move: usize,
}

impl Default for History {
    fn default() -> Self {
        let mut history = Self {
            history: Vec::new(),
            current_move: 0,
        };
        history.add_info(Default::default(), Default::default());
        history
    }
}

impl History {
    fn add_info(&mut self, state: BoardState, mv: Move) {
        self.history.push((state, mv));
        self.current_move += 1
    }

    fn get_current_state_mut(&mut self) -> &mut BoardState {
        &mut self.history[self.current_move - 1].0
    }

    fn get_current_state(&self) -> &BoardState {
        &self.history.last().unwrap().0
    }

    fn resume(&mut self) {
        self.current_move = self.history.len()
    }

    fn previous_state(&mut self) {
        if self.current_move > 1 {
            self.current_move -= 1
        }
    }

    fn next_state(&mut self) {
        if self.current_move < self.history.len() {
            self.current_move += 1
        }
    }
}

#[derive(Clone, Default)]
pub struct Game {
    valid_moves: HashSet<Move>,
    status: GameStatus,
    history: History,
}

impl Game {
    pub fn new() -> Self {
        let mut game = Self::default();
        game.add_moves();
        game
    }

    pub fn get_piece(&self, position: &Position) -> Option<Piece> {
        self.history.get_current_state().get_piece(position)
    }

    pub fn get_board_state(&self) -> &BoardState {
        self.history.get_current_state()
    }

    pub fn go_back_a_turn(&mut self) {
        self.history.previous_state()
    }

    pub fn go_forward_a_turn(&mut self) {
        self.history.next_state()
    }

    pub fn resume(&mut self) {
        self.history.resume()
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult {
        if let Some(piece) = self.history.get_current_state().get_piece(&from) {
            let mv = Move::new(from, to);

            self.is_move_valid(&mv)?;
            self.history.get_current_state_mut().move_piece(&mv);
            self.update(&mv);

            println!("{} : {}", piece, mv);
        }
        Ok(())
    }

    fn update(&mut self, mv: &Move) {
        self.add_moves();
        self.remove_self_checks();
        self.update_status();
        self.history
            .add_info(self.history.get_current_state().clone(), *mv)
    }

    fn update_status(&mut self) {
        let mut next_turn = self.clone();
        let next_turn_state = next_turn.history.get_current_state_mut();
        next_turn_state.player = !next_turn_state.player;
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
        self.valid_moves.iter().any(|m| {
            self.get_piece(&m.to) == Some(Piece::King(!self.history.get_current_state().player))
        })
    }

    fn remove_self_checks(&mut self) {
        for mv in self.valid_moves.clone() {
            let mut next_turn = self.clone();
            next_turn.history.get_current_state_mut().move_piece(&mv);
            next_turn.add_moves();
            if next_turn.has_check() {
                self.valid_moves.remove(&mv);
            }
        }
    }

    fn is_move_valid(&self, mv: &Move) -> ChessResult {
        BoardState::is_in_bounds(&mv.from)?;
        BoardState::is_in_bounds(&mv.to)?;
        self.history.get_current_state().is_piece_some(&mv.from)?;

        if self.valid_moves.contains(mv) {
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }

    fn add_pawn_advance_moves(&mut self, from: Position) {
        let v = Displacement::get_pawn_advance_vector(self.history.get_current_state().player);
        let mut to = from + v;
        if BoardState::is_in_bounds(&to).is_ok()
            && self.history.get_current_state().get_piece(&to).is_none()
        {
            self.valid_moves.insert(Move { from, to });
            to += v;
            if self.history.get_current_state().get_piece(&to).is_none()
                && self.can_double_move(&from)
            {
                self.valid_moves.insert(Move { from, to });
            }
        }
    }

    fn can_double_move(&self, from: &Position) -> bool {
        if let Piece::Pawn(player) = self.history.get_current_state().get_piece(from).unwrap() {
            return match player {
                Player::White => from.y == 1,
                Player::Black => from.y == 6,
            };
        }
        false
    }

    fn add_pawn_capture_moves(&mut self, from: Position) {
        for &v in Displacement::get_pawn_capture_vectors(self.history.get_current_state().player) {
            let to = from + v;
            if BoardState::is_in_bounds(&to).is_ok() {
                if let Some(piece) = self.history.get_current_state().get_piece(&to) {
                    if piece.get_player() != self.history.get_current_state().player {
                        self.valid_moves.insert(Move::new(from, to));
                    }
                }
                if Some(to) == self.history.get_current_state().en_passant_position {
                    self.valid_moves.insert(Move::new(from, to));
                }
            }
        }
    }

    fn add_moves_for_piece(&mut self, from: Position) {
        if let Some(piece) = self.history.get_current_state().get_piece(&from) {
            if piece.get_player() == self.history.get_current_state().player {
                if piece.is_pawn() {
                    self.add_pawn_advance_moves(from);
                    self.add_pawn_capture_moves(from);
                } else {
                    for &v in piece.get_vectors() {
                        let mut to = from + v;
                        while BoardState::is_in_bounds(&to).is_ok() {
                            if let Some(piece) = self.history.get_current_state().get_piece(&to) {
                                if piece.get_player() != self.history.get_current_state().player {
                                    self.valid_moves.insert(Move { from, to });
                                }
                                break;
                            }
                            self.valid_moves.insert(Move { from, to });
                            if !self
                                .history
                                .get_current_state()
                                .get_piece(&from)
                                .unwrap()
                                .can_snipe()
                            {
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
            CastlingRights::get_castling_info(self.history.get_current_state().player);

        if self
            .history
            .get_current_state()
            .castling_rights
            .has_castling_right(kingside)
            && !(1..=2).any(|i| {
                self.history
                    .get_current_state()
                    .has_piece(&(king_square + Displacement::RIGHT * i))
            })
        {
            self.valid_moves.insert(Move {
                from: king_square,
                to: king_square + Displacement::RIGHT * 2,
            });
        }

        if self
            .history
            .get_current_state()
            .castling_rights
            .has_castling_right(queenside)
            && !(1..=3).any(|i| {
                self.history
                    .get_current_state()
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
        self.history.get_current_state().has_piece(position)
    }
}
