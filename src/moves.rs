use crate::pieces::Player;
use std::ops::Mul;

#[derive(Clone, Copy)]
pub struct Move {
    pub dx: i8,
    pub dy: i8,
}

impl Move {
    const QUEEN_MOVES: [Self; 8] = [
        Self { dx: 1, dy: 0 },
        Self { dx: -1, dy: 0 },
        Self { dx: 0, dy: 1 },
        Self { dx: 0, dy: -1 },
        Self { dx: 1, dy: 1 },
        Self { dx: 1, dy: -1 },
        Self { dx: -1, dy: 1 },
        Self { dx: -1, dy: -1 },
    ];
    const KNIGHT_MOVES: [Self; 8] = [
        Self { dx: 1, dy: 2 },
        Self { dx: 1, dy: -2 },
        Self { dx: -1, dy: 2 },
        Self { dx: -1, dy: -2 },
        Self { dx: 2, dy: 1 },
        Self { dx: 2, dy: -1 },
        Self { dx: -2, dy: 1 },
        Self { dx: -2, dy: -1 },
    ];

    pub fn get_pawn_advance_move(player: Player) -> Self {
        let dx;
        let dy;

        match player {
            Player::White => (dx, dy) = (0, -1),
            Player::Black => (dx, dy) = (0, 1),
        }

        Self { dx, dy }
    }

    pub fn get_pawn_capture_moves_white() -> &'static [Self] {
        &[Self { dx: -1, dy: -1 }, Self { dx: 1, dy: -1 }]
    }

    pub fn get_pawn_capture_moves_black() -> &'static [Self] {
        &[Self { dx: 1, dy: 1 }, Self { dx: -1, dy: 1 }]
    }

    pub fn get_queen_moves() -> &'static [Self] {
        &Self::QUEEN_MOVES
    }

    pub fn get_king_moves() -> &'static [Self] {
        &Self::QUEEN_MOVES
    }

    pub fn get_rook_moves() -> &'static [Self] {
        &Self::QUEEN_MOVES[0..4]
    }

    pub fn get_bishop_moves() -> &'static [Self] {
        &Self::QUEEN_MOVES[4..8]
    }

    pub fn get_knight_moves() -> &'static [Self] {
        &Self::KNIGHT_MOVES
    }
}

impl Mul<i8> for Move {
    type Output = Self;

    fn mul(self, rhs: i8) -> Self::Output {
        Self {
            dx: self.dx * rhs,
            dy: self.dy * rhs,
        }
    }
}
