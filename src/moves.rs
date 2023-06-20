use crate::pieces::Position;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: Position,
    pub to: Position,
}
