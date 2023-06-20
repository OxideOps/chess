use crate::pieces::Player;
use std::ops::Mul;

#[derive(Clone, Copy)]
pub struct Displacement {
    pub dx: i8,
    pub dy: i8,
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

    pub fn get_pawn_advance_vectors(player: Player) -> Self {
        match player {
            Player::White => Self { dx: 0, dy: 1 },
            Player::Black => Self { dx: 0, dy: -1 },
        }
    }

    pub fn get_white_pawn_capture_vectors() -> &'static [Self] {
        &[Self { dx: 1, dy: 1 }, Self { dx: -1, dy: 1 }]
    }

    pub fn get_black_pawn_capture_vectors() -> &'static [Self] {
        &[Self { dx: -1, dy: -1 }, Self { dx: 1, dy: -1 }]
    }

    pub fn get_queen_moves() -> &'static [Self] {
        &Self::QUEEN_VECTORS
    }

    pub fn get_king_moves() -> &'static [Self] {
        &Self::QUEEN_VECTORS
    }

    pub fn get_rook_moves() -> &'static [Self] {
        &Self::QUEEN_VECTORS[0..4]
    }

    pub fn get_bishop_moves() -> &'static [Self] {
        &Self::QUEEN_VECTORS[4..8]
    }

    pub fn get_knight_moves() -> &'static [Self] {
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
