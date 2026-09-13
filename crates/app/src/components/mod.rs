mod board;
mod controls;
mod engine_panel;
mod eval_bar;
mod import_panel;
mod move_list;

pub use board::Board;
pub use controls::Controls;
pub use engine_panel::EnginePanel;
pub use eval_bar::EvalBar;
pub use import_panel::ImportPanel;
pub use move_list::MoveList;

use chess_core::{Color, GameStatus};

pub fn side_name(color: Color) -> &'static str {
    if color.is_white() { "White" } else { "Black" }
}

/// One line describing the viewed position, for a status bar.
pub fn status_text(status: GameStatus, turn: Color) -> String {
    match status {
        GameStatus::Ongoing => format!("{} to move", side_name(turn)),
        GameStatus::Check => format!("{} to move — check", side_name(turn)),
        GameStatus::Checkmate { winner } => format!("Checkmate — {} wins", side_name(winner)),
        GameStatus::Stalemate => "Draw by stalemate".into(),
        GameStatus::InsufficientMaterial => "Draw by insufficient material".into(),
        GameStatus::FiftyMoveRule => "Draw by the fifty-move rule".into(),
        GameStatus::ThreefoldRepetition => "Draw by threefold repetition".into(),
    }
}
