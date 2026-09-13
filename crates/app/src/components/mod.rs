mod board;
mod controls;
mod move_list;

pub use board::Board;
pub use controls::Controls;
pub use move_list::MoveList;

use chess_core::Color;

pub fn side_name(color: Color) -> &'static str {
    if color.is_white() { "White" } else { "Black" }
}
