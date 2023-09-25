use crate::{board_state::BoardState, moves::Move, game_status::GameStatus};

use std::fmt;

#[derive(Clone, Copy, Default)]
pub struct Turn {
    pub(super) board_state: BoardState,
    pub(super) mv: Move,
    pub(super) piece_captured: bool,
    pub(super) status: GameStatus,
}

impl Turn {
    pub fn new(
        board_state: BoardState,
        mv: Move,
        piece_captured: bool,
    ) -> Self {
        Self {
            board_state,
            mv,
            piece_captured,
            ..Default::default()
        }
    }
}

impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.board_state.get_piece(&self.mv.to).unwrap(),
            if self.piece_captured { "x" } else { "" },
            self.mv.to,
            if matches!(self.status, GameStatus::Check(..)) { 
                "+" 
            } else if matches!(self.status, GameStatus::Checkmate(..)) { 
                "#" 
            } else {
                ""
            }
        )
    }
}
