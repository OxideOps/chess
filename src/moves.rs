use crate::pieces::Position;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: Position,
    pub to: Position,
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let files = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        let ranks = ['1', '2', '3', '4', '5', '6', '7', '8'];

        let from_file = files[self.from.x];
        let to_file = files[self.to.x];

        let from_rank = ranks[8 - self.from.y - 1];
        let to_rank = ranks[8 - self.to.y - 1];

        write!(f, "{}{}->{}{}", from_file, from_rank, to_file, to_rank)
    }
}
