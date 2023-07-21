use crate::board::BoardState;
use crate::castling_rights::{CastlingRights, CastlingRightsKind};
use crate::displacement::Displacement;
use crate::history::History;
use crate::moves::Move;
use crate::pieces::{Color, Piece, Position};
use crate::timer::Timer;

use std::collections::HashSet;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use web_time::Duration;

pub type ChessResult = Result<(), ChessError>;
#[derive(Debug)]
pub enum ChessError {
    OutOfBounds,
    NoPieceAtPosition,
    InvalidMove,
    OwnPieceInDestination,
    ColorInCheck,
    Checkmate,
    Stalemate,
    InvalidPromotion,
    NotColorsTurn,
    EmptyPieceMove,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum GameStatus {
    #[default]
    Ongoing,
    Stalemate,
    Check,
    Checkmate,
    Replay,
}

impl GameStatus {
    pub fn update(&mut self, status: GameStatus) {
        if *self != status {
            log::info!("GameStatus changing from {:?} to {:?}", *self, status);
            *self = status
        }
    }
}

#[derive(Clone)]
pub struct Game {
    valid_moves: HashSet<Move>,
    pub status: GameStatus,
    history: History,
    timer: Timer,
}

impl Default for Game {
    fn default() -> Self {
        Self::with_state(BoardState::default())
    }
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_history(history: History) -> Self {
        let mut game = Self {
            valid_moves: HashSet::default(),
            status: GameStatus::default(),
            timer: Timer::with_duration(Duration::from_secs(3600)),
            history,
        };
        game.add_moves();
        game
    }

    pub fn with_state(state: BoardState) -> Self {
        Self::with_history(History::with_state(state))
    }

    pub fn get_piece(&self, position: &Position) -> Option<Piece> {
        self.get_current_state().get_piece(position)
    }

    fn get_current_state(&self) -> &BoardState {
        self.history.get_current_state()
    }

    fn clone_current_state(&self) -> BoardState {
        self.history.clone_current_state()
    }

    pub fn get_current_player(&self) -> Color {
        self.get_current_state().player
    }

    fn is_piece_some(&self, at: &Position) -> ChessResult {
        self.get_current_state().is_piece_some(at)
    }

    pub fn has_piece(&self, position: &Position) -> bool {
        self.get_current_state().has_piece(position)
    }

    fn piece_can_snipe(&self, at: &Position) -> bool {
        self.get_piece(at).unwrap().can_snipe()
    }

    fn get_info_for_turn(&self, mv: usize) -> &(BoardState, Move) {
        self.history.get_info_for_move(mv)
    }

    fn has_castling_right(&self, right: CastlingRightsKind) -> bool {
        self.get_current_state()
            .castling_rights
            .has_castling_right(right)
    }

    fn navigate_history(&mut self, change: impl FnOnce(&mut History)) {
        change(&mut self.history);
        self.update_status();
    }

    pub fn go_back_a_move(&mut self) {
        self.navigate_history(|history| history.previous_move());
    }

    pub fn go_forward_a_move(&mut self) {
        self.navigate_history(|history| history.next_move());
    }

    pub fn go_to_start(&mut self) {
        self.navigate_history(|history| history.go_to_start());
    }

    pub fn resume(&mut self) {
        self.navigate_history(|history| history.resume());
    }

    fn add_info(&mut self, next_state: BoardState, mv: Move) {
        self.history.add_info(next_state, mv);
    }

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult {
        if let Some(piece) = self.get_piece(&from) {
            let mv = Move::new(from, to);
            self.is_move_valid(&mv)?;

            let mut next_state = self.clone_current_state();
            next_state.move_piece(&mv);
            self.history.add_info(next_state, mv);

            log::info!("{} : {}", piece, mv);
            self.update();
        }
        Ok(())
    }

    fn update(&mut self) {
        self.add_moves();
        self.remove_self_checks();
        self.update_status();
        self.update_timer();
    }

    fn update_timer(&mut self) {
        if !self.timer.is_active() {
            self.timer.start()
        }
        self.timer.print();
        self.timer.next_player();
    }

    fn update_status(&mut self) {
        if self.history.is_replaying() {
            self.status.update(GameStatus::Replay);
            return;
        }
        let king_is_under_attack = self.is_king_under_attack();
        let valid_moves_is_empty = self.valid_moves.is_empty();

        if !king_is_under_attack && valid_moves_is_empty {
            self.status.update(GameStatus::Stalemate)
        } else if king_is_under_attack && valid_moves_is_empty {
            self.status.update(GameStatus::Checkmate)
        } else if king_is_under_attack {
            self.status.update(GameStatus::Check)
        } else {
            self.status.update(GameStatus::Ongoing)
        }
    }

    fn is_attacking_king(&self) -> bool {
        self.valid_moves
            .iter()
            .any(|mv| self.get_piece(&mv.to) == Some(Piece::King(!self.get_current_player())))
    }

    fn is_king_under_attack(&self) -> bool {
        let mut enemy_board = self.clone_current_state();
        enemy_board.player = !enemy_board.player;
        Self::with_state(enemy_board).is_attacking_king()
    }

    fn remove_self_checks(&mut self) {
        let current_board = self.clone_current_state();
        self.valid_moves.retain(|mv| {
            let mut future_board = current_board.clone();
            future_board.move_piece(mv);
            !Self::with_state(future_board).is_attacking_king()
        })
    }

    fn is_move_valid(&self, mv: &Move) -> ChessResult {
        BoardState::is_in_bounds(&mv.from)?;
        BoardState::is_in_bounds(&mv.to)?;
        self.is_piece_some(&mv.from)?;

        if self.valid_moves.contains(mv) {
            Ok(())
        } else {
            Err(ChessError::InvalidMove)
        }
    }

    fn add_pawn_advance_moves(&mut self, from: &Position) {
        let v = Displacement::get_pawn_advance_vector(self.get_current_player());
        let mut to = *from + v;
        if BoardState::is_in_bounds(&to).is_ok() && self.get_piece(&to).is_none() {
            self.valid_moves.insert(Move::new(*from, to));
            to += v;
            if self.can_double_move(from) && self.get_piece(&to).is_none() {
                self.valid_moves.insert(Move::new(*from, to));
            }
        }
    }

    fn can_double_move(&self, from: &Position) -> bool {
        if let Piece::Pawn(player) = self.get_piece(from).unwrap() {
            return match player {
                Color::White => from.y == 1,
                Color::Black => from.y == 6,
            };
        }
        false
    }

    fn add_pawn_capture_moves(&mut self, from: &Position) {
        for &v in Displacement::get_pawn_capture_vectors(self.get_current_player()) {
            let to = *from + v;
            if BoardState::is_in_bounds(&to).is_ok() {
                if let Some(piece) = self.get_piece(&to) {
                    if piece.get_player() != self.get_current_player() {
                        self.valid_moves.insert(Move::new(*from, to));
                    }
                }
                if Some(to) == self.get_current_state().en_passant_position {
                    self.valid_moves.insert(Move::new(*from, to));
                }
            }
        }
    }

    fn add_moves_for_piece(&mut self, from: &Position) {
        if let Some(piece) = self.get_piece(from) {
            if piece.get_player() == self.get_current_player() {
                if piece.is_pawn() {
                    self.add_pawn_advance_moves(from);
                    self.add_pawn_capture_moves(from);
                } else {
                    for &v in piece.get_vectors() {
                        let mut to = *from + v;
                        while BoardState::is_in_bounds(&to).is_ok() {
                            if let Some(piece) = self.get_piece(&to) {
                                if piece.get_player() != self.get_current_player() {
                                    self.valid_moves.insert(Move::new(*from, to));
                                }
                                break;
                            }
                            self.valid_moves.insert(Move::new(*from, to));
                            if !self.piece_can_snipe(from) {
                                break;
                            }
                            to += v;
                        }
                    }
                }
            }
        }
    }

    fn add_moves(&mut self) {
        self.valid_moves.clear();
        for y in 0..8 {
            for x in 0..8 {
                self.add_moves_for_piece(&Position::new(x, y))
            }
        }
        self.add_castling_moves()
    }

    fn add_castling_moves(&mut self) {
        let (king_square, kingside, queenside) =
            CastlingRights::get_castling_info(self.get_current_player());

        if self.has_castling_right(kingside)
            && !(1..=2).any(|i| self.has_piece(&(king_square + Displacement::RIGHT * i)))
        {
            self.valid_moves.insert(Move::new(
                king_square,
                king_square + Displacement::RIGHT * 2,
            ));
        }

        if self.has_castling_right(queenside)
            && !(1..=3).any(|i| self.has_piece(&(king_square + Displacement::LEFT * i)))
        {
            self.valid_moves
                .insert(Move::new(king_square, king_square + Displacement::LEFT * 2));
        }
    }

    pub fn get_real_state_hash(&self) -> u64 {
        self.history.get_real_state().get_hash()
    }
}
