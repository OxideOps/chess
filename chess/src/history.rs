use std::collections::HashMap;

use crate::board_state::BoardState;
use crate::moves::Move;
use crate::turn::Turn;

#[derive(Clone, PartialEq)]
pub struct History {
    pub(super) turns: Vec<Turn>,
    pub(super) repetition_counter: HashMap<BoardState, usize>,
    current_turn: usize,
    fifty_move_count: u8,
    initial_state: BoardState,
}

impl History {
    pub fn with_state(initial_state: BoardState) -> Self {
        Self {
            initial_state,
            repetition_counter: vec![(initial_state, 1)].into_iter().collect(),
            ..Default::default()
        }
    }

    pub fn get_real_turn(&self) -> Turn {
        if let Some(&turn) = self.turns.last() {
            turn
        } else {
            Turn::default()
        }
    }

    pub fn get_current_turn(&self) -> usize {
        self.current_turn
    }

    pub fn get_board_state(&self, turn: usize) -> &BoardState {
        if turn == 0 {
            &self.initial_state
        } else {
            &self.turns[turn - 1].board_state
        }
    }

    pub fn get_real_state_repetition_count(&self) -> usize {
        *self.repetition_counter.get(self.get_real_state()).unwrap()
    }

    pub fn update_fifty_move_info(&mut self, piece_captured: bool) {
        if piece_captured {
            self.fifty_move_count += 1;
        } else {
            self.fifty_move_count = 0;
        }
    }

    pub fn update_repetition_info(&mut self, next_state: BoardState) {
        *self.repetition_counter.entry(next_state).or_insert(0) += 1;
    }

    pub fn add_turn(&mut self, turn: Turn) {
        self.turns.push(turn);
        self.current_turn += 1
    }

    pub fn add_info(&mut self, next_state: BoardState, mv: Move, king_is_checked: bool) {
        let real_state = *self.get_real_state();
        let is_pawn = real_state.get_piece(&mv.from).unwrap().is_pawn();
        let is_capture_move =
            real_state.get_piece(&mv.to).is_some() || (is_pawn && mv.from.x != mv.to.x);

        self.update_fifty_move_info(is_capture_move);
        self.update_repetition_info(next_state);
        self.add_turn(Turn::new(next_state, mv, is_capture_move, king_is_checked));
    }

    pub fn get_fifty_move_count(&self) -> u8 {
        self.fifty_move_count / 2
    }

    pub fn get_current_state(&self) -> &BoardState {
        self.get_board_state(self.current_turn)
    }

    pub fn get_real_state(&self) -> &BoardState {
        self.get_board_state(self.turns.len())
    }

    pub fn get_info_for_move(&self, turn: usize) -> &Turn {
        &self.turns[turn]
    }

    pub fn resume(&mut self) {
        self.current_turn = self.turns.len()
    }

    pub fn previous_move(&mut self) {
        if self.current_turn > 0 {
            self.current_turn -= 1
        }
    }

    pub fn next_move(&mut self) {
        if self.current_turn < self.turns.len() {
            self.current_turn += 1
        }
    }

    pub fn go_to_start(&mut self) {
        self.current_turn = 0
    }

    pub fn is_replaying(&self) -> bool {
        self.current_turn != self.turns.len()
    }
    pub fn get_current_round(&self) -> usize {
        (self.current_turn + 1) / 2
    }
}

impl Default for History {
    fn default() -> Self {
        let (turns, current_turn, fifty_move_count, initial_state) = Default::default();
        Self {
            turns,
            current_turn,
            fifty_move_count,
            initial_state,
            repetition_counter: vec![(BoardState::default(), 1)].into_iter().collect(),
        }
    }
}
