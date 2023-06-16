#[derive(Clone, Copy)]
pub struct Move {
    pub dx: i8,
    pub dy: i8,
}

impl Move {
    const QUEEN_MOVES: [Move; 8] = [
        Move { dx: 1, dy: 0 },
        Move { dx: -1, dy: 0 },
        Move { dx: 0, dy: 1 },
        Move { dx: 0, dy: -1 },
        Move { dx: 1, dy: 1 },
        Move { dx: 1, dy: -1 },
        Move { dx: -1, dy: 1 },
        Move { dx: -1, dy: -1 },
    ];
    const KNIGHT_MOVES: [Move; 8] = [
        Move { dx: 1, dy: 2 },
        Move { dx: 1, dy: -2 },
        Move { dx: -1, dy: 2 },
        Move { dx: -1, dy: -2 },
        Move { dx: 2, dy: 1 },
        Move { dx: 2, dy: -1 },
        Move { dx: -2, dy: 1 },
        Move { dx: -2, dy: -1 },
    ];

    pub fn get_pawn_advance_moves_black() -> Move {
        Self::QUEEN_MOVES[2]
    }

    pub fn get_pawn_advance_moves_white() -> Move {
        Self::QUEEN_MOVES[3]
    }

    pub fn get_pawn_capture_moves_black() -> &'static [Move] {
        &Self::QUEEN_MOVES[4..6]
    }

    pub fn get_pawn_capture_moves_white() -> &'static [Move] {
        &Self::QUEEN_MOVES[6..8]
    }
    pub fn get_queen_moves() -> &'static [Move] {
        &Self::QUEEN_MOVES
    }

    pub fn get_king_moves() -> &'static [Move] {
        &Self::QUEEN_MOVES
    }

    pub fn get_rook_moves() -> &'static [Move] {
        &Self::QUEEN_MOVES[0..4]
    }

    pub fn get_bishop_moves() -> &'static [Move] {
        &Self::QUEEN_MOVES[4..8]
    }

    pub fn get_knight_moves() -> &'static [Move] {
        &Self::KNIGHT_MOVES
    }
}
