use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Default, Hash, Serialize, Deserialize)]
pub enum Color {
    #[default]
    White,
    Black,
}

impl std::ops::Not for Color {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}
