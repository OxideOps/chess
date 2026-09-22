//! Lessons: short drills played against the engine from set positions, and
//! the rules for when a drill is won or lost.
//!
//! The client plays the engine's side; this module only knows the drills
//! and judges the moves made so far, so the rules are the same wherever
//! they run and are tested here.

use serde::{Deserialize, Serialize};
use shakmaty::Color;
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{Game, GameError, GameStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum Goal {
    /// Mate the lone king within the move budget.
    Checkmate,
    /// Promote a pawn (or mate) within the move budget.
    Promote,
    /// Don't lose: survive the move budget, or reach any draw.
    Draw,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Drill {
    /// Stable for good: finished lessons are stored under it, so never rename
    /// or reuse one (see `drill_ids_are_stable`).
    pub id: String,
    pub title: String,
    /// One line, for the list of lessons.
    pub summary: String,
    /// The technique, in a few sentences.
    pub lesson: String,
    pub fen: String,
    #[serde(with = "crate::protocol::color")]
    #[cfg_attr(feature = "ts", ts(type = "\"white\" | \"black\""))]
    pub student: Color,
    pub goal: Goal,
    /// The student's move budget (for `Draw`, how many moves to survive).
    pub moves: u32,
}

/// Where a drill stands after the moves so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export, rename = "DrillStatus"))]
pub enum Status {
    Going { moves_left: u32 },
    Won { reason: String },
    Lost { reason: String },
}

// Positional so the table in `drills` reads as one row per drill.
#[allow(clippy::too_many_arguments)]
fn drill(
    id: &str,
    title: &str,
    summary: &str,
    lesson: &str,
    fen: &str,
    student: Color,
    goal: Goal,
    moves: u32,
) -> Drill {
    Drill {
        id: id.into(),
        title: title.into(),
        summary: summary.into(),
        lesson: lesson.into(),
        fen: fen.into(),
        student,
        goal,
        moves,
    }
}

/// Every drill, easiest first.
pub fn drills() -> Vec<Drill> {
    vec![
        drill(
            "back-rank-mate",
            "Back-rank mate",
            "Your first checkmate, in one move.",
            "A king stuck behind its own pawns can be mated on the back rank. Find the check \
             that nothing can block and that leaves the king nowhere to go.",
            "6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1",
            Color::White,
            Goal::Checkmate,
            1,
        ),
        drill(
            "two-rooks",
            "Two rooks: the ladder",
            "Mate a lone king with two rooks.",
            "The rooks take turns: one checks the king along a rank while the other guards the \
             rank beside it, pushing the king to the edge a rank at a time. Keep the rooks far \
             from the king so it can't attack them. Your own king isn't needed.",
            "8/8/3k4/8/8/8/8/R3K2R w - - 0 1",
            Color::White,
            Goal::Checkmate,
            15,
        ),
        drill(
            "queen-mate",
            "King and queen",
            "Mate with the queen, helped by your king.",
            "Use the queen to box the king in from a knight's move away, shrinking its space \
             without giving check. Once it's on the edge, walk your king up to support the \
             queen and mate. Watch for stalemate: leave the lone king a move until the last.",
            "8/8/8/3k4/8/8/7Q/4K3 w - - 0 1",
            Color::White,
            Goal::Checkmate,
            20,
        ),
        drill(
            "rook-mate",
            "King and rook",
            "The classic: mate with a single rook.",
            "The rook cuts the king off along a rank or file. Bring your king up until the \
             kings face each other with one square between; then a rook check drives the king \
             back a rank. When the kings don't face each other, make a waiting move with the \
             rook. Repeat until the edge.",
            "8/8/8/3k4/8/8/8/R3K3 w - - 0 1",
            Color::White,
            Goal::Checkmate,
            30,
        ),
        drill(
            "king-in-front",
            "King in front of the pawn",
            "Win a king and pawn ending.",
            "With your king on the sixth rank in front of its pawn, the pawn queens whoever is \
             to move. Keep the king ahead of the pawn and take the opposition, facing the \
             enemy king with a square between, so it has to step aside.",
            "4k3/8/4K3/4P3/8/8/8/8 w - - 0 1",
            Color::White,
            Goal::Promote,
            12,
        ),
        drill(
            "hold-the-draw",
            "Hold the draw",
            "Defend a king against king and pawn.",
            "Stay in front of the pawn. When the kings face each other with a square between, \
             the side not to move has the opposition, so answer each king move to keep it. \
             When the pawn advances, step straight back. On the last ranks, go to the square in \
             front of the pawn: the ending is stalemate or a lost pawn.",
            "8/8/4k3/8/4K3/4P3/8/8 w - - 0 1",
            Color::Black,
            Goal::Draw,
            20,
        ),
    ]
}

pub fn find(id: &str) -> Option<Drill> {
    drills().into_iter().find(|d| d.id == id)
}

fn moves_text(n: u32) -> String {
    format!("{n} {}", if n == 1 { "move" } else { "moves" })
}

fn draw_reason(status: GameStatus) -> Option<&'static str> {
    match status {
        GameStatus::Stalemate => Some("stalemate"),
        GameStatus::InsufficientMaterial => Some("insufficient material"),
        GameStatus::ThreefoldRepetition => Some("threefold repetition"),
        GameStatus::FiftyMoveRule => Some("the fifty-move rule"),
        _ => None,
    }
}

/// Judge the drill after `moves` (UCI, both sides, from the drill's FEN).
pub fn assess(drill: &Drill, moves: &[&str]) -> Result<Status, GameError> {
    let mut game = Game::from_fen(&drill.fen)?;
    let mut student_moves = 0;
    let (mut student_promoted, mut opponent_promoted) = (false, false);
    for uci in moves {
        let mover = game.turn();
        let played = game.play_uci(uci)?;
        let promoted = played.mv.promotion().is_some();
        if mover == drill.student {
            student_moves += 1;
            student_promoted |= promoted;
        } else {
            opponent_promoted |= promoted;
        }
    }
    let status = game.final_status();
    let budget_spent = student_moves >= drill.moves;
    let lost = |reason: String| Ok(Status::Lost { reason });
    let won = |reason: String| Ok(Status::Won { reason });
    match drill.goal {
        Goal::Checkmate | Goal::Promote => {
            if let GameStatus::Checkmate { winner } = status {
                return if winner == drill.student {
                    won("Checkmate!".into())
                } else {
                    lost("You were checkmated.".into())
                };
            }
            if drill.goal == Goal::Promote && student_promoted {
                return won("Promoted! The new queen wins easily.".into());
            }
            if let Some(why) = draw_reason(status) {
                return lost(format!("A draw by {why}: the win slipped away."));
            }
            if budget_spent {
                let what = if drill.goal == Goal::Checkmate {
                    "mate"
                } else {
                    "promote"
                };
                return lost(format!(
                    "Out of moves: the goal was to {what} within {}.",
                    moves_text(drill.moves)
                ));
            }
        }
        Goal::Draw => {
            if let GameStatus::Checkmate { winner } = status
                && winner != drill.student
            {
                return lost("Checkmated.".into());
            }
            if opponent_promoted {
                return lost("The pawn promoted.".into());
            }
            if let Some(why) = draw_reason(status) {
                return won(format!("A draw by {why}."));
            }
            if budget_spent {
                return won(format!(
                    "You held for {}: that's a draw.",
                    moves_text(drill.moves)
                ));
            }
        }
    }
    Ok(Status::Going {
        moves_left: drill.moves - student_moves,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(fen: &str, student: Color, goal: Goal, moves: u32) -> Drill {
        drill("t", "t", "t", "t", fen, student, goal, moves)
    }

    #[test]
    fn every_drill_is_sound() {
        let all = drills();
        assert_eq!(all.len(), 6);
        let mut ids: Vec<_> = all.iter().map(|d| d.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len(), "ids are unique");
        for d in &all {
            let game = Game::from_fen(&d.fen).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            assert!(!game.final_status().is_game_over(), "{}", d.id);
            assert_eq!(
                assess(d, &[]).unwrap(),
                Status::Going {
                    moves_left: d.moves
                },
                "{}",
                d.id
            );
            assert_eq!(find(&d.id).as_ref(), Some(d));
        }
        // Mating drills start with the student to move; the defence starts with the engine.
        assert_eq!(
            Game::from_fen(&find("queen-mate").unwrap().fen)
                .unwrap()
                .turn(),
            Color::White
        );
        let hold = find("hold-the-draw").unwrap();
        assert_ne!(Game::from_fen(&hold.fen).unwrap().turn(), hold.student);
        assert_eq!(find("nope"), None);
    }

    /// Accounts store their finished lessons under these ids (the server's
    /// `lesson_completions` table, and browsers' `localStorage`), so an id
    /// is forever: reorder the drills or add new ones, but never rename or
    /// reuse an id, or someone's progress silently goes missing.
    #[test]
    fn drill_ids_are_stable() {
        let ids: Vec<_> = drills().into_iter().map(|d| d.id).collect();
        for id in [
            "back-rank-mate",
            "hold-the-draw",
            "king-in-front",
            "queen-mate",
            "rook-mate",
            "two-rooks",
        ] {
            assert!(ids.iter().any(|i| i == id), "{id} was renamed or removed");
        }
    }

    #[test]
    fn the_first_drill_is_mate_in_one() {
        let d = find("back-rank-mate").unwrap();
        assert_eq!(
            assess(&d, &["a1a8"]).unwrap(),
            Status::Won {
                reason: "Checkmate!".into()
            }
        );
        assert_eq!(
            assess(&d, &["a1a7"]).unwrap(),
            Status::Lost {
                reason: "Out of moves: the goal was to mate within 1 move.".into()
            }
        );
    }

    #[test]
    fn checkmate_drills_are_lost_to_stalemate_and_the_budget() {
        // Qg7# mates; Qg6 stalemates.
        let d = custom(
            "7k/8/5K2/8/8/8/8/6Q1 w - - 0 1",
            Color::White,
            Goal::Checkmate,
            3,
        );
        assert!(matches!(assess(&d, &["g1g7"]).unwrap(), Status::Won { .. }));
        assert_eq!(
            assess(&d, &["g1g6"]).unwrap(),
            Status::Lost {
                reason: "A draw by stalemate: the win slipped away.".into()
            }
        );
        assert_eq!(
            assess(&d, &["g1g2", "h8h7"]).unwrap(),
            Status::Going { moves_left: 2 }
        );
        assert!(matches!(
            assess(&d, &["g1g2", "h8h7", "g2g3", "h7h8", "g3g2"]).unwrap(),
            Status::Lost { .. }
        ));
        assert!(assess(&d, &["g1g8", "h8g8"]).is_ok()); // queen lost: insufficient material
        assert!(matches!(
            assess(&d, &["g1g8", "h8g8"]).unwrap(),
            Status::Lost { reason } if reason.contains("insufficient material")
        ));
        assert!(assess(&d, &["e2e4"]).is_err());
    }

    #[test]
    fn promoting_wins_a_promotion_drill() {
        let d = custom(
            "8/3KP3/8/8/8/8/8/7k w - - 0 1",
            Color::White,
            Goal::Promote,
            5,
        );
        assert_eq!(
            assess(&d, &["e7e8q"]).unwrap(),
            Status::Won {
                reason: "Promoted! The new queen wins easily.".into()
            }
        );
        assert_eq!(
            assess(&d, &["d7d6"]).unwrap(),
            Status::Going { moves_left: 4 }
        );
    }

    #[test]
    fn a_draw_drill_is_won_by_surviving_or_any_draw() {
        // Black defends; White's pawn is one step from queening.
        let d = custom(
            "8/4P3/8/8/8/2k5/8/K7 w - - 0 1",
            Color::Black,
            Goal::Draw,
            2,
        );
        assert_eq!(
            assess(&d, &["e7e8q"]).unwrap(),
            Status::Lost {
                reason: "The pawn promoted.".into()
            }
        );
        // Black captures the pawn: insufficient material is a draw, which is a win here.
        let d = custom("8/8/8/8/8/8/4Pk2/K7 b - - 0 1", Color::Black, Goal::Draw, 5);
        assert_eq!(
            assess(&d, &["f2e2"]).unwrap(),
            Status::Won {
                reason: "A draw by insufficient material.".into()
            }
        );
        // Surviving the budget.
        let d = find("hold-the-draw").unwrap();
        let held = ["e4d4", "e6d6"];
        assert_eq!(assess(&d, &held).unwrap(), Status::Going { moves_left: 19 });
        let short = Drill { moves: 1, ..d };
        assert_eq!(
            assess(&short, &held).unwrap(),
            Status::Won {
                reason: "You held for 1 move: that's a draw.".into()
            }
        );
    }
}
