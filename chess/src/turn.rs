use crate::{board_state::BoardState, moves::Move};

use std::fmt;

#[derive(Clone, Default)]
pub struct Turn {
    pub(super) board_state: BoardState,
    pub(super) mv: Move,
    pub(super) piece_captured: bool,
}

impl Turn {
    pub fn new(board_state: BoardState, mv: Move, piece_captured: bool) -> Self {
        Self {
            board_state,
            mv,
            piece_captured,
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
        write!(
            f,
            "{}{}{}",
            if self.piece_captured { "x" } else { "" },
            self.board_state.get_piece(&self.mv.to).unwrap(),
            self.mv.to
        )
    }
}
