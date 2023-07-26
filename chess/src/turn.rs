use crate::{board::BoardState, moves::Move};

use std::fmt;

#[derive(Clone, Default)]
pub struct Turn {
    pub board_state: BoardState,
    pub mv: Move,
}

impl Turn {
    pub fn new(board_state: BoardState, mv: Move) -> Self {
        Self {
            board_state,
            mv
        }
    }
    pub fn with_state(board_state: BoardState) -> Self {
        Self {
            board_state,
            ..Default::default()
        }
    } 
}

impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.board_state.get_piece(&self.mv.to).unwrap(), self.mv.to)
    }
}