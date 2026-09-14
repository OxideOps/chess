//! Checking puzzle solutions.
//!
//! A puzzle, as the Lichess puzzle database publishes them, is a FEN and a
//! list of UCI moves. The first move is the opponent's, which sets the puzzle
//! up; then the solver's moves and the opponent's replies alternate, ending
//! with the solver's. The solver must find each of their moves, except that
//! any move that checkmates also solves the puzzle (Lichess's rule).

use serde::{Deserialize, Serialize};
use shakmaty::{CastlingMode, Chess, Color, Move, Position, fen::Fen, san::SanPlus, uci::UciMove};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::GameError;

#[derive(Debug, Clone)]
pub struct Puzzle {
    /// The position after the setup move: where the solver starts.
    start: Chess,
    setup: Move,
    solution: Vec<Move>,
}

/// What the solver's latest move means.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export, rename = "PuzzleVerdict"))]
pub enum Verdict {
    /// Right, and the puzzle goes on: the opponent answers with `reply` (UCI).
    Correct { reply: String },
    /// That move finishes the puzzle.
    Solved,
    /// Not it. `expected` is the move that was wanted, in UCI and SAN.
    Wrong {
        expected: String,
        expected_san: String,
    },
}

fn parse(position: &Chess, uci: &str) -> Result<Move, GameError> {
    let parsed: UciMove = uci
        .parse()
        .map_err(|_| GameError::InvalidUci(uci.to_string()))?;
    parsed.to_move(position).map_err(|_| GameError::IllegalMove)
}

fn uci(mv: &Move) -> String {
    UciMove::from_move(*mv, CastlingMode::Standard).to_string()
}

impl Puzzle {
    /// Check the puzzle replays: the FEN is valid and every move is legal in
    /// turn, and the solver has the last move.
    pub fn new(fen: &str, moves: &[&str]) -> Result<Puzzle, GameError> {
        let fen: Fen = fen
            .parse()
            .map_err(|_| GameError::InvalidFen(fen.to_string()))?;
        let mut position: Chess = fen
            .into_position(CastlingMode::Standard)
            .map_err(|e| GameError::InvalidFen(e.to_string()))?;
        // Setup, then an odd number of moves: the solver's, replies, the solver's.
        if moves.len() < 2 || !moves.len().is_multiple_of(2) {
            return Err(GameError::InvalidUci(format!(
                "a puzzle needs a setup move and an odd number of solution moves, got {} moves",
                moves.len()
            )));
        }
        let setup = parse(&position, moves[0])?;
        position.play_unchecked(setup);
        let start = position.clone();
        let mut solution = Vec::with_capacity(moves.len() - 1);
        for m in &moves[1..] {
            let mv = parse(&position, m)?;
            position.play_unchecked(mv);
            solution.push(mv);
        }
        Ok(Puzzle {
            start,
            setup,
            solution,
        })
    }

    /// The side the solver plays.
    pub fn solver(&self) -> Color {
        self.start.turn()
    }

    /// The opponent's move that sets the puzzle up, in UCI.
    pub fn setup(&self) -> String {
        uci(&self.setup)
    }

    /// Solution moves in UCI: the solver's, the replies, the solver's.
    pub fn solution(&self) -> Vec<String> {
        self.solution.iter().map(uci).collect()
    }

    /// Judge `played`: every move made after the setup move, the last being
    /// the solver's newest. Replies in it must be the ones this puzzle gave.
    pub fn judge(&self, played: &[&str]) -> Result<Verdict, GameError> {
        if played.len().is_multiple_of(2) {
            return Err(GameError::InvalidUci(
                "judge the solver's move: `played` must end with it".to_string(),
            ));
        }
        let mut position = self.start.clone();
        for (i, m) in played.iter().enumerate() {
            let mv = parse(&position, m)?;
            let expected = self.solution.get(i).ok_or(GameError::IllegalMove)?;
            let solvers_move = i % 2 == 0;
            if mv != *expected {
                if !solvers_move {
                    // A reply we didn't give: the caller is out of step.
                    return Err(GameError::IllegalMove);
                }
                let mut after = position.clone();
                after.play_unchecked(mv);
                if after.is_checkmate() {
                    return Ok(Verdict::Solved);
                }
                return Ok(Verdict::Wrong {
                    expected: uci(expected),
                    expected_san: SanPlus::from_move(position, *expected).to_string(),
                });
            }
            position.play_unchecked(mv);
        }
        Ok(match self.solution.get(played.len()) {
            Some(reply) => Verdict::Correct { reply: uci(reply) },
            None => Verdict::Solved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lichess puzzle 00008 (CC0): White to find Rxe7, Nc1, Qxc1.
    fn lichess_00008() -> Puzzle {
        Puzzle::new(
            "r6k/pp2r2p/4Rp1Q/3p4/8/1N1P2R1/PqP2bPP/7K b - - 0 24",
            &["f2g3", "e6e7", "b2b1", "b3c1", "b1c1", "h6c1"],
        )
        .unwrap()
    }

    #[test]
    fn walks_a_real_puzzle_to_the_end() {
        let p = lichess_00008();
        assert_eq!(p.solver(), Color::White);
        assert_eq!(p.setup(), "f2g3");
        assert_eq!(p.solution(), ["e6e7", "b2b1", "b3c1", "b1c1", "h6c1"]);
        assert_eq!(
            p.judge(&["e6e7"]),
            Ok(Verdict::Correct {
                reply: "b2b1".into()
            })
        );
        assert_eq!(
            p.judge(&["e6e7", "b2b1", "b3c1"]),
            Ok(Verdict::Correct {
                reply: "b1c1".into()
            })
        );
        assert_eq!(
            p.judge(&["e6e7", "b2b1", "b3c1", "b1c1", "h6c1"]),
            Ok(Verdict::Solved)
        );
    }

    #[test]
    fn a_wrong_move_names_the_right_one() {
        let p = lichess_00008();
        assert_eq!(
            p.judge(&["h6h7"]),
            Ok(Verdict::Wrong {
                expected: "e6e7".into(),
                expected_san: "Rxe7".into()
            })
        );
        assert_eq!(
            p.judge(&["e6e7", "b2b1", "h6c1"]),
            Ok(Verdict::Wrong {
                expected: "b3c1".into(),
                expected_san: "Nc1".into()
            })
        );
    }

    #[test]
    fn any_checkmate_solves() {
        // After ...b6, both Ra8# and Re8# mate; the puzzle lists Re8#.
        let p = Puzzle::new(
            "6k1/1p3ppp/8/8/8/8/5PPP/R3R1K1 b - - 0 1",
            &["b7b6", "e1e8"],
        )
        .unwrap();
        assert_eq!(p.judge(&["e1e8"]), Ok(Verdict::Solved));
        assert_eq!(p.judge(&["a1a8"]), Ok(Verdict::Solved));
        assert!(matches!(p.judge(&["e1e7"]), Ok(Verdict::Wrong { .. })));
    }

    #[test]
    fn rejects_malformed_puzzles_and_calls() {
        assert!(Puzzle::new("not a fen", &["e2e4", "e7e5"]).is_err());
        let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        assert!(Puzzle::new(start, &["e2e4"]).is_err()); // no solution
        assert!(Puzzle::new(start, &["e2e4", "e7e5", "g1f3"]).is_err()); // ends on a reply
        assert!(Puzzle::new(start, &["e2e5", "e7e5"]).is_err()); // illegal
        let p = lichess_00008();
        assert!(p.judge(&[]).is_err());
        assert!(p.judge(&["e6e7", "b2b1"]).is_err()); // ends on a reply
        assert!(p.judge(&["e6e7", "h8g8", "b3c1"]).is_err()); // a reply we didn't give
        assert!(p.judge(&["e6e6"]).is_err()); // not a legal move
    }
}
