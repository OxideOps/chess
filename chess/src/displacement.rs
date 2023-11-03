use std::ops::Mul;

use crate::color::Color;

#[derive(Clone, Copy)]
pub(super) struct Displacement {
    pub(super) dx: i8,
    pub(super) dy: i8,
}

impl Displacement {
    const QUEEN_VECTORS: [Self; 8] = [
        Self { dx: 1, dy: 0 },
        Self { dx: -1, dy: 0 },
        Self { dx: 0, dy: 1 },
        Self { dx: 0, dy: -1 },
        Self { dx: 1, dy: 1 },
        Self { dx: 1, dy: -1 },
        Self { dx: -1, dy: 1 },
        Self { dx: -1, dy: -1 },
    ];
    const KNIGHT_VECTORS: [Self; 8] = [
        Self { dx: 1, dy: 2 },
        Self { dx: 1, dy: -2 },
        Self { dx: -1, dy: 2 },
        Self { dx: -1, dy: -2 },
        Self { dx: 2, dy: 1 },
        Self { dx: 2, dy: -1 },
        Self { dx: -2, dy: 1 },
        Self { dx: -2, dy: -1 },
    ];
    pub(super) const RIGHT: Self = Self { dx: 1, dy: 0 };
    pub(super) const LEFT: Self = Self { dx: -1, dy: 0 };

    pub(super) fn get_pawn_advance_vector(player: Color) -> Self {
        match player {
            Color::White => Self { dx: 0, dy: 1 },
            Color::Black => Self { dx: 0, dy: -1 },
        }
    }

    pub(super) fn get_pawn_capture_vectors(player: Color) -> &'static [Self] {
        match player {
            Color::White => &[Self { dx: 1, dy: 1 }, Self { dx: -1, dy: 1 }],
            Color::Black => &[Self { dx: -1, dy: -1 }, Self { dx: 1, dy: -1 }],
        }
    }

    pub(super) fn get_queen_vectors() -> &'static [Self] {
        &Self::QUEEN_VECTORS
    }

    pub(super) fn get_king_vectors() -> &'static [Self] {
        &Self::QUEEN_VECTORS
    }

    pub(super) fn get_rook_vectors() -> &'static [Self] {
        &Self::QUEEN_VECTORS[0..4]
    }

    pub(super) fn get_bishop_vectors() -> &'static [Self] {
        &Self::QUEEN_VECTORS[4..8]
    }

    pub(super) fn get_knight_vectors() -> &'static [Self] {
        &Self::KNIGHT_VECTORS
    }
}

impl Mul<i8> for Displacement {
    type Output = Self;

    fn mul(self, rhs: i8) -> Self::Output {
        Self {
            dx: self.dx * rhs,
            dy: self.dy * rhs,
        }
    }
}
