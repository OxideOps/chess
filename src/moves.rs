use crate::pieces::Position;
use std::fmt;

/// A struct representing a move in a game. A move consists of a starting
/// position (`from`) and an ending position (`to`).
///
/// This struct implements the `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, and `Hash` traits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    /// The starting position of the move.
    pub from: Position,
    /// The ending position of the move.
    pub to: Position,
}

impl Move {
    /// Returns a new `Move` which is the inverse of the current one. That is,
    /// the `from` and `to` fields of the new `Move` are swapped compared to
    /// the current one.
    ///
    /// # Example
    ///
    /// ```
    /// use chess::{moves::Move, pieces::Position};
    ///
    /// let move_1 = Move { from: Position { x: 1, y: 2 }, to: Position { x: 3, y: 4 } };
    /// let move_2 = move_1.inverse();
    /// assert_eq!(move_1.from, move_2.to);
    /// assert_eq!(move_1.to, move_2.from);
    /// ```
    pub fn inverse(&self) -> Self {
        Self {
            from: self.to,
            to: self.from,
        }
    }
}

impl fmt::Display for Move {
    /// Formats the `Move` for display. The starting and ending positions are displayed
    /// as coordinates on an 8x8 grid, where the x-coordinates are letters from 'a' to 'h'
    /// and the y-coordinates are numbers from '1' to '8'.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let files = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        let ranks = ['1', '2', '3', '4', '5', '6', '7', '8'];

        let from_file = files[self.from.x];
        let to_file = files[self.to.x];

        let from_rank = ranks[self.from.y];
        let to_rank = ranks[self.to.y];

        write!(f, "{}{} -> {}{}", from_file, from_rank, to_file, to_rank)
    }
}
