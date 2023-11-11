use std::fmt;

use chess::Color;

// How much differences in stockfish evaluation affect the alpha of the arrows
const ALPHA_SENSITIVITY: f64 = 1.0 / 30.0;
// How much a mate in 1 is worth in centipawns
const MATE_IN_1_EVAL: f64 = 100000.0;
// How much (in centipawns) getting a mate 1 move sooner is worth
const MATE_MOVE_EVAL: f64 = 50.0;

#[derive(Clone, Copy)]
pub enum Eval {
    Centipawns(i32),
    Mate(i32),
}

impl Eval {
    pub(super) fn change_perspective(&mut self) {
        match self {
            Eval::Centipawns(cp) => *cp = -*cp,
            Eval::Mate(mate) => *mate = -*mate,
        }
    }

    pub(super) fn to_score(self) -> f64 {
        ALPHA_SENSITIVITY
            * match self {
                Eval::Centipawns(cp) => cp as f64,
                Eval::Mate(mate) => {
                    if mate >= 0 {
                        MATE_IN_1_EVAL - MATE_MOVE_EVAL * (mate - 1) as f64
                    } else {
                        -MATE_IN_1_EVAL - MATE_MOVE_EVAL * (mate + 1) as f64
                    }
                }
            }
    }

    pub(crate) fn get_winning_player(self) -> Color {
        if self.to_score() > 0.0 {
            Color::White
        } else {
            Color::Black
        }
    }
}

impl fmt::Display for Eval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Eval::Centipawns(cp) => write!(f, "{:.1}", cp.abs() as f64 / 100.0),
            Eval::Mate(mate) => write!(f, "M{}", mate.abs()),
        }
    }
}
