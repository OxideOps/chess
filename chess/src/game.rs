use crate::board_state::BoardState;
use crate::castling_rights::{CastlingRights, CastlingRightsKind};
use crate::chess_result::{ChessError, ChessResult};
use crate::color::Color;
use crate::displacement::Displacement;
use crate::game_status::{DrawKind, GameStatus};
use crate::history::History;
use crate::moves::Move;
use crate::piece::Piece;
use crate::position::Position;
use crate::round_info::RoundInfo;
use crate::timer::Timer;
use std::collections::HashSet;
use web_time::Duration;

#[derive(Default, Clone)]
pub struct Game {
    valid_moves: HashSet<Move>,
    pub status: GameStatus,
    history: History,
    pub timer: Timer,
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> GameBuilder {
        GameBuilder::default()
    }

    pub fn with_state(state: BoardState) -> Self {
        Game::builder().state(state).build()
    }

    pub fn get_piece(&self, position: &Position) -> Option<Piece> {
        self.get_current_state().get_piece(position)
    }

    fn get_current_state(&self) -> &BoardState {
        self.history.get_current_state()
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

    fn has_castling_right(&self, right: CastlingRightsKind) -> bool {
        self.get_current_state()
            .castling_rights
            .has_castling_right(right)
    }

    fn navigate_history(&mut self, navigate: impl FnOnce(&mut History)) {
        navigate(&mut self.history);
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

    pub fn move_piece(&mut self, from: Position, to: Position) -> ChessResult {
        if let Some(piece) = self.get_piece(&from) {
            let mv = Move::new(from, to);
            self.is_move_valid(&mv)?;
            let mut next_state = *self.get_current_state();
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

    fn check_for_draw(&mut self) -> bool {
        if self.history.get_fifty_move_count() == 50 {
            self.status
                .update(GameStatus::Draw(DrawKind::FiftyMoveRule));
            return true;
        }
        if self.history.get_real_state_repetition_count() == 3 {
            self.status.update(GameStatus::Draw(DrawKind::Repetition));
            return true;
        }
        false
    }

    fn update_status(&mut self) {
        if self.history.is_replaying() {
            self.status.update(GameStatus::Replay);
            return;
        }
        if self.get_active_time().is_zero() {
            self.status.update(GameStatus::Timeout);
            return;
        }
        if self.check_for_draw() {
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
        let mut enemy_board = *self.get_current_state();
        enemy_board.player = !enemy_board.player;
        Self::with_state(enemy_board).is_attacking_king()
    }

    fn moves_into_check(mut board_state: BoardState, mv: &Move) -> bool {
        board_state.move_piece(mv);
        Self::with_state(board_state).is_attacking_king()
    }

    fn remove_self_checks(&mut self) {
        let current_state = *self.get_current_state();
        self.valid_moves
            .retain(|mv| !Self::moves_into_check(current_state, mv))
    }

    pub fn is_move_valid(&self, mv: &Move) -> ChessResult {
        if self.get_active_time().is_zero() {
            return Err(ChessError::Timeout);
        }
        if matches!(self.status, GameStatus::Draw(..)) {
            return Err(ChessError::GameIsInDraw);
        }
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

    pub fn add_moves(&mut self) {
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
            && !Self::moves_into_check(
                *self.get_current_state(),
                &Move::new(king_square, king_square + Displacement::RIGHT),
            )
            && !(1..=2).any(|i| self.has_piece(&(king_square + Displacement::RIGHT * i)))
        {
            self.valid_moves.insert(Move::new(
                king_square,
                king_square + Displacement::RIGHT * 2,
            ));
        }

        if self.has_castling_right(queenside)
            && !Self::moves_into_check(
                *self.get_current_state(),
                &Move::new(king_square, king_square + Displacement::LEFT),
            )
            && !(1..=3).any(|i| self.has_piece(&(king_square + Displacement::LEFT * i)))
        {
            self.valid_moves
                .insert(Move::new(king_square, king_square + Displacement::LEFT * 2));
        }
    }

    pub fn get_real_state_hash(&self) -> u64 {
        self.history.get_real_state().get_hash()
    }

    pub fn get_active_time(&self) -> Duration {
        self.timer.get_active_time()
    }

    pub fn is_timer_active(&self) -> bool {
        self.timer.is_active()
    }

    pub fn get_time(&self, player: Color) -> Duration {
        self.timer.get_time(player)
    }

    pub fn get_pieces(&self) -> Vec<(Piece, Position)> {
        let mut pieces: Vec<(Piece, Position)> = vec![];
        for x in 0..8 {
            for y in 0..8 {
                let pos = Position::new(x, y);
                if let Some(piece) = self.get_piece(&pos) {
                    pieces.push((piece, pos));
                }
            }
        }
        pieces
    }

    pub fn get_move_history(&self) -> Vec<String> {
        self.history
            .turns
            .iter()
            .map(|turn| format!("{turn}"))
            .collect()
    }

    pub fn get_rounds_info(&self) -> Vec<RoundInfo> {
        self.history
            .turns
            .chunks(2)
            .map(|turns| RoundInfo {
                white_string: format!("{}", turns[0]),
                black_string: turns
                    .get(1)
                    .map_or("...".to_string(), |black_turn| format!("{black_turn}")),
            })
            .collect()
    }

    pub fn get_current_round(&self) -> usize {
        self.history.get_current_round()
    }

    pub fn get_current_turn(&self) -> usize {
        self.history.get_current_turn()
    }

    pub fn get_real_player(&self) -> Color {
        self.history.get_real_state().player
    }

    pub fn trigger_timeout(&mut self) {
        self.timer.stop();
        self.update_status();
    }
}
pub struct GameBuilder {
    duration: Duration,
    state: BoardState,
}

impl Default for GameBuilder {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(60),
            state: BoardState::default(),
        }
    }
}

impl GameBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> Game {
        let mut game = Game {
            history: History::with_state(self.state),
            timer: Timer::with_duration(self.duration),
            ..Default::default()
        };
        game.add_moves();
        game
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn state(mut self, state: BoardState) -> Self {
        self.state = state;
        self
    }

    pub fn player(mut self, player: Color) -> Self {
        self.state.player = player;
        self
    }
}
