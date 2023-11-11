use std::collections::HashSet;

use web_time::Duration;

use crate::{
    board_state::BoardState,
    castling_rights::{CastlingRights, CastlingRightsKind},
    color::Color,
    displacement::Displacement,
    game_status::{DrawKind, GameStatus},
    history::History,
    moves::Move,
    piece::Piece,
    position::Position,
    result::{ChessError, ChessResult},
    round_info::RoundInfo,
    timer::{Timer, DEFAULT_DURATION},
    turn::Turn,
};

const MAX_FEN_STR: usize = 87;

#[derive(Clone)]
pub struct Game {
    valid_moves: HashSet<Move>,
    pub(super) status: GameStatus,
    history: History,
    timer: Timer,
}

impl Default for Game {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    fn builder() -> GameBuilder {
        GameBuilder::new()
    }

    pub fn with_start_time(start_time: Duration) -> Self {
        GameBuilder::new().start_time(start_time).build()
    }

    fn with_state(state: BoardState) -> Self {
        Self::builder().state(state).build()
    }

    pub fn is_replaying(&self) -> bool {
        self.history.is_replaying()
    }

    pub fn is_in_check(&self) -> bool {
        matches!(self.status, GameStatus::Check(..))
    }

    pub fn reset(&mut self) {
        *self = Self::new()
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

    pub(super) fn has_piece(&self, position: &Position) -> bool {
        self.get_current_state().has_piece(position)
    }

    fn piece_can_snipe(&self, at: &Position) -> bool {
        self.get_piece(at).map_or(false, |piece| piece.can_snipe())
    }

    fn has_castling_right(&self, right: CastlingRightsKind) -> bool {
        self.get_current_state()
            .castling_rights
            .has_castling_right(right)
    }

    pub fn get_valid_destinations_for_piece(&self, position: &Position) -> Vec<Position> {
        self.valid_moves
            .iter()
            .filter(|mv| mv.from == *position)
            .map(|mv| mv.to)
            .collect()
    }

    fn navigate_history(&mut self, navigate: impl FnOnce(&mut History)) {
        navigate(&mut self.history);
        self.add_moves();
        self.remove_self_checks();
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
        self.history.update_status(self.status);
        self.update_timer();
    }

    fn update_timer(&mut self) {
        if self.status.is_game_over() {
            self.timer.stop();
            return;
        }
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
        if self.get_active_time().is_zero() {
            self.status
                .update(GameStatus::Timeout(self.get_real_player()));
            return;
        }
        if self.check_for_draw() {
            return;
        }
        let king_is_under_attack = Self::is_king_under_attack(
            &self
                .history
                .get_real_turn()
                .unwrap_or(&Turn::default())
                .board_state,
        );
        let valid_moves_is_empty = self.valid_moves.is_empty();

        if !king_is_under_attack && valid_moves_is_empty {
            self.status.update(GameStatus::Draw(DrawKind::Stalemate))
        } else if king_is_under_attack && valid_moves_is_empty {
            self.status
                .update(GameStatus::Checkmate(self.get_real_player()))
        } else if king_is_under_attack {
            self.status
                .update(GameStatus::Check(self.get_real_player()))
        } else if !self.history.turns.is_empty() {
            self.status.update(GameStatus::Ongoing)
        }
    }

    fn is_attacking_king(&self) -> bool {
        self.valid_moves
            .iter()
            .any(|mv| self.get_piece(&mv.to) == Some(Piece::King(!self.get_current_player())))
    }

    fn is_king_under_attack(board_state: &BoardState) -> bool {
        let mut enemy_board = *board_state;
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
        if let Some(Piece::Pawn(player)) = self.get_piece(from) {
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

    pub(super) fn get_current_turn_index(&self) -> usize {
        self.history.get_current_turn_index()
    }

    pub fn get_real_player(&self) -> Color {
        self.history.get_real_state().player
    }

    pub fn trigger_timeout(&mut self) {
        self.timer.stop();
        self.update_status();
    }

    pub fn get_fen_str(&self) -> String {
        let mut fen = String::with_capacity(MAX_FEN_STR);
        let mut empty_count = 0;
        for y in (0..8).rev() {
            for x in 0..8 {
                if let Some(piece) = self.get_piece(&Position { x, y }) {
                    if empty_count > 0 {
                        fen.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    fen.push(piece.get_fen_char());
                } else {
                    empty_count += 1;
                }
            }
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
                empty_count = 0;
            }
            fen.push('/');
        }
        fen.push_str(&format!(
            " {} {} {} {} {}",
            self.get_current_player().get_fen_char(),
            self.get_current_state().castling_rights.get_fen_str(),
            self.get_current_state()
                .en_passant_position
                .map_or("-".to_string(), |pos| pos.to_string()),
            self.history.fifty_move_count,
            self.get_current_turn_index() / 2 + 1
        ));
        fen
    }

    pub fn get_current_move(&self) -> Option<Move> {
        self.history.get_current_move()
    }

    pub fn get_highlighted_squares_info(&self) -> Vec<(Position, String)> {
        const MOVED_CLASS: &str = "moved-square";
        const CHECK_CLASS: &str = "check-square";
        const CHECKMATE_CLASS: &str = "checkmate-square";

        let mut info: Vec<(Position, String)> = vec![];

        // from-to square of current move
        if let Some(mv) = &self.get_current_move() {
            info.push((mv.from, MOVED_CLASS.into()));
            info.push((mv.to, MOVED_CLASS.into()));
        }

        // highlight king depending on status
        if let Some(turn) = self.history.get_current_turn() {
            match turn.status {
                GameStatus::Check(color) => match color {
                    Color::White => {
                        info.push((turn.board_state.white_king_position, CHECK_CLASS.into()))
                    }
                    Color::Black => {
                        info.push((turn.board_state.black_king_position, CHECK_CLASS.into()))
                    }
                },
                GameStatus::Checkmate(color) => {
                    match color {
                        Color::White => info
                            .push((turn.board_state.white_king_position, CHECKMATE_CLASS.into())),
                        Color::Black => info
                            .push((turn.board_state.black_king_position, CHECKMATE_CLASS.into())),
                    }
                }
                _ => (),
            }
        }
        info
    }

    pub fn game_over(&self) -> bool {
        matches!(
            self.status,
            GameStatus::Checkmate(..) | GameStatus::Draw(..)
        )
    }
}
struct GameBuilder {
    start_time: Duration,
    state: BoardState,
}

impl Default for GameBuilder {
    fn default() -> Self {
        Self {
            start_time: DEFAULT_DURATION,
            state: BoardState::default(),
        }
    }
}

impl GameBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> Game {
        let mut game = Game {
            valid_moves: HashSet::default(),
            history: History::with_state(self.state),
            timer: Timer::with_duration(self.start_time),
            status: GameStatus::default(),
        };
        game.add_moves();
        game
    }

    fn start_time(mut self, start_time: Duration) -> Self {
        self.start_time = start_time;
        self
    }

    fn state(mut self, state: BoardState) -> Self {
        self.state = state;
        self
    }
}
