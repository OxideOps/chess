#[derive(Clone, Copy, PartialEq, Debug, Default, Hash)]
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
