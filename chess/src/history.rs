use std::collections::HashMap;

use crate::board_state::BoardState;
use crate::game_status::GameStatus;
use crate::moves::Move;
use crate::turn::Turn;

#[derive(Clone)]
pub(super) struct History {
    pub(super) turns: Vec<Turn>,
    pub(super) repetition_counter: HashMap<BoardState, usize>,
    current_turn_index: usize,
    pub(super) fifty_move_count: u8,
    initial_state: BoardState,
}

impl History {
    pub(super) fn with_state(initial_state: BoardState) -> Self {
        Self {
            initial_state,
            repetition_counter: vec![(initial_state, 1)].into_iter().collect(),
            ..Default::default()
        }
    }

    pub(super) fn update_status(&mut self, status: GameStatus) {
        self.get_real_turn().unwrap().status = status;
    }

    pub(super) fn get_real_turn(&mut self) -> Option<&mut Turn> {
        self.turns.last_mut()
    }

    pub(super) fn get_current_move(&self) -> Option<Move> {
        if self.get_current_turn_index() == 0 {
            None
        } else {
            Some(self.turns[self.get_current_turn_index() - 1].mv)
        }
    }

    pub(super) fn get_current_turn(&self) -> Option<Turn> {
        if self.get_current_turn_index() == 0 {
            None
        } else {
            Some(self.turns[self.get_current_turn_index() - 1])
        }
    }

    pub(super) fn get_current_turn_index(&self) -> usize {
        self.current_turn_index
    }

    pub(super) fn get_board_state(&self, turn: usize) -> &BoardState {
        if turn == 0 {
            &self.initial_state
        } else {
            &self.turns[turn - 1].board_state
        }
    }

    pub(super) fn get_real_state_repetition_count(&self) -> usize {
        *self.repetition_counter.get(self.get_real_state()).unwrap()
    }

    fn update_fifty_move_info(&mut self, piece_captured: bool, pawn_moved: bool) {
        if piece_captured || pawn_moved {
            self.fifty_move_count = 0
        } else {
            self.fifty_move_count += 1;
        }
    }

    fn update_repetition_info(&mut self, next_state: BoardState) {
        *self.repetition_counter.entry(next_state).or_insert(0) += 1;
    }

    fn add_turn(&mut self, turn: Turn) {
        self.turns.push(turn);
        self.current_turn_index += 1
    }

    pub(super) fn add_info(&mut self, next_state: BoardState, mv: Move) {
        let real_state = *self.get_real_state();
        let is_pawn = real_state.get_piece(&mv.from).unwrap().is_pawn();
        let is_capture_move =
            real_state.get_piece(&mv.to).is_some() || (is_pawn && mv.from.x != mv.to.x);

        self.update_fifty_move_info(is_capture_move, is_pawn);
        self.update_repetition_info(next_state);
        self.add_turn(Turn::new(next_state, mv, is_capture_move));
    }

    pub(super) fn get_fifty_move_count(&self) -> u8 {
        self.fifty_move_count / 2
    }

    pub(super) fn get_current_state(&self) -> &BoardState {
        self.get_board_state(self.current_turn_index)
    }

    pub(super) fn get_real_state(&self) -> &BoardState {
        self.get_board_state(self.turns.len())
    }

    pub(super) fn resume(&mut self) {
        self.current_turn_index = self.turns.len()
    }

    pub(super) fn previous_move(&mut self) {
        if self.current_turn_index > 0 {
            self.current_turn_index -= 1
        }
    }

    pub(super) fn next_move(&mut self) {
        if self.current_turn_index < self.turns.len() {
            self.current_turn_index += 1
        }
    }

    pub(super) fn go_to_start(&mut self) {
        self.current_turn_index = 0
    }

    pub(super) fn is_replaying(&self) -> bool {
        self.current_turn_index != self.turns.len()
    }
    pub(super) fn get_current_round(&self) -> usize {
        (self.current_turn_index + 1) / 2
    }
}

impl Default for History {
    fn default() -> Self {
        let (turns, current_turn_index, fifty_move_count, initial_state) = Default::default();
        Self {
            turns,
            current_turn_index,
            fifty_move_count,
            initial_state,
            repetition_counter: vec![(BoardState::default(), 1)].into_iter().collect(),
        }
    }
}
