use crate::board::BoardState;
use crate::moves::Move;

#[derive(Clone, Default)]
pub struct History {
    history: Vec<(BoardState, Move)>,
    current_move: usize,
}

impl History {
    pub fn with_state(state: BoardState) -> Self {
        Self {
            history: vec![(state, Move::default())],
            ..Default::default()
        }
    }

    pub fn add_info(&mut self, next_state: BoardState, mv: Move) {
        self.history.push((next_state, mv));
        self.current_move += 1
    }

    pub fn get_current_state(&self) -> &BoardState {
        &self.history[self.current_move].0
    }

    pub fn clone_current_state(&self) -> BoardState {
        self.get_current_state().clone()
    }

    pub fn get_real_state(&self) -> &BoardState {
        let last = self.history.last();
        &last.unwrap().0
    }

    pub fn get_info_for_move(&self, turn: usize) -> &(BoardState, Move) {
        &self.history[turn]
    }

    pub fn resume(&mut self) {
        self.current_move = self.history.len() - 1
    }

    pub fn previous_move(&mut self) {
        if self.current_move > 0 {
            self.current_move -= 1
        }
    }

    pub fn next_move(&mut self) {
        if self.current_move < self.history.len() - 1 {
            self.current_move += 1
        }
    }

    pub fn go_to_start(&mut self) {
        self.current_move = 0
    }

    pub fn is_replaying(&self) -> bool {
        self.current_move != self.history.len() - 1
    }

    pub fn current_turn(&self) -> usize {
        self.current_move / 2 + 1
    }
}
