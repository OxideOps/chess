use crate::board::BoardState;
use crate::moves::Move;
use crate::turn::Turn;

#[derive(Clone, Default)]
pub struct History {
    pub turns: Vec<Turn>,
    current_turn: usize,
    pub fifty_move_counter: u8,
}

impl History {
    pub fn with_state(state: BoardState) -> Self {
        Self {
            turns: vec![Turn::with_state(state)],
            ..Default::default()
        }
    }

    pub fn add_info(&mut self, next_state: BoardState, mv: Move) {
        self.turns.push(Turn::new(next_state, mv));
        self.current_turn += 1;
            self.fifty_move_counter += 1;
    }

    pub fn get_current_state(&self) -> &BoardState {
        &self.turns[self.current_turn].board_state
    }

    pub fn clone_current_state(&self) -> BoardState {
        self.get_current_state().clone()
    }

    pub fn get_real_state(&self) -> &BoardState {
        &self.turns.last().unwrap().board_state
    }

    pub fn get_info_for_move(&self, turn: usize) -> &Turn {
        &self.turns[turn]
    }

    pub fn resume(&mut self) {
        self.current_turn = self.turns.len() - 1
    }

    pub fn previous_move(&mut self) {
        if self.current_turn > 0 {
            self.current_turn -= 1
        }
    }

    pub fn next_move(&mut self) {
        if self.current_turn < self.turns.len() - 1 {
            self.current_turn += 1
        }
    }

    pub fn go_to_start(&mut self) {
        self.current_turn = 0
    }

    pub fn is_replaying(&self) -> bool {
        self.current_turn != self.turns.len() - 1
    }

    pub fn current_round(&self) -> usize {
        self.current_turn / 2 + 1
    }
}
