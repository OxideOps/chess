//! Lessons: short drills played against the engine from set positions, and
//! the rules for when a drill is won or lost.
//!
//! The client plays the engine's side; this module only knows the drills
//! and judges the moves made so far, so the rules are the same wherever
//! they run and are tested here.
//!
//! Material is counted the usual way (pawn 1, knight and bishop 3, rook 5,
//! queen 9) and always against the drill's start. A capture only counts once
//! the side that lost the piece has had a move to take something back: what
//! the student wins is judged with the student to move (after the engine's
//! answer), what the student loses is judged after the student's own move.

use serde::{Deserialize, Serialize};
use shakmaty::{Chess, Color, Position, Role};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::facts::{home_square, material_of};
use crate::{Game, GameError, GameStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum Goal {
    /// Mate the lone king within the move budget.
    Checkmate,
    /// Promote a pawn (or mate) within the move budget.
    Promote,
    /// Don't lose: survive the move budget, or reach any draw, without
    /// losing material or letting a pawn promote.
    Draw,
    /// Be at least `points` of material up on the start with the student
    /// to move (or mate) within the move budget. Material won on the last
    /// move of the budget counts once the engine has answered it.
    WinMaterial { points: u32 },
    /// Within the move budget: castle, get every knight and bishop off its
    /// starting square, and be no material down, all with the student to move.
    Develop,
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

const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// Every drill, easiest first: the first mate, the opening, tactics, the
/// basic mates, then the endings. The order is the route through the
/// material; ids are what progress is stored under, so they never change.
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
            "develop-and-castle",
            "Develop and castle",
            "Open a game: pieces out, king safe.",
            "Bring the knights and bishops out before anything else, push a centre pawn or two \
             to let the bishops through, and castle to tuck the king away. Move each piece once \
             where you can, and leave the queen at home, where Stockfish can't chase it and gain \
             time. Nothing may be left hanging: a pawn down when you finish is a drill lost.",
            START,
            Color::White,
            Goal::Develop,
            10,
        ),
        drill(
            "early-queen",
            "Meet the early queen",
            "Stop the four-move mate, then out-develop it.",
            "White's queen and bishop both aim at f7, a square only your king defends: 3...Nf6?? \
             walks into Qxf7 mate. Cover f7 first (3...g6 blocks the queen and makes room for a \
             bishop on g7; 3...Qe7 also holds), then keep developing. Once f7 is safe, a queen \
             out this early is a target: every move it spends running is one you spend \
             developing.",
            "r1bqkbnr/pppp1ppp/2n5/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 3 3",
            Color::Black,
            Goal::Develop,
            10,
        ),
        drill(
            "back-rank-combination",
            "Back-rank combination",
            "Mate in two: more attackers than defenders.",
            "Count who attacks e8 and who defends it: your queen and rook, lined up on the \
             e-file, against Black's queen. With the king boxed in by its own pawns, whoever \
             makes the last capture on e8 gives mate, so start the captures, even though the \
             first one gives up your queen.",
            "4r1k1/pppq1ppp/8/8/8/8/PPP1QPPP/4R1K1 w - - 0 1",
            Color::White,
            Goal::Checkmate,
            2,
        ),
        drill(
            "knight-fork",
            "Knight fork",
            "Attack the king and the queen at once.",
            "A knight attacks squares no other piece covers, so one knight move can check the \
             king and hit another piece too. Find the check that also attacks the queen; after \
             the king steps away, take it. A queen for a knight is a winning trade.",
            "r5k1/pp1n1ppp/2q5/3N4/8/8/PP2QPPP/3R2K1 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 6 },
            2,
        ),
        drill(
            "pin",
            "Pin",
            "Pin the queen to its king.",
            "A piece standing in front of its king can't step off the line between them. Put a \
             defended bishop on the diagonal that runs through Black's queen to the king: the \
             queen can only take the bishop and be taken back, or be lost for nothing.",
            "1r4k1/p5pp/5p2/3q4/8/1P6/P4PPP/2R2BK1 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 6 },
            2,
        ),
        drill(
            "skewer",
            "Skewer",
            "Check the king, take what stands behind it.",
            "A skewer is a pin the other way round: attack the valuable piece in front, and when \
             it moves out of the line, take the one behind. Here the king and queen share a \
             diagonal with nothing between them. Check along it; the king has to step aside.",
            "q7/5p1p/6p1/3k4/8/8/4BPPP/6K1 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 6 },
            2,
        ),
        drill(
            "discovered-attack",
            "Discovered attack",
            "Move one piece to unmask another.",
            "Your bishop stands between your queen and Black's on the d-file. Move it with a \
             threat of its own, best of all a check, and Black has no time to save the queen: \
             the check must be answered first, then your queen takes.",
            "r1b2rk1/pp3ppp/2pq1n2/8/8/3B1N2/PPP2PPP/R2Q1RK1 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 6 },
            2,
        ),
        drill(
            "deflection",
            "Deflection",
            "Lure a defender away from its job.",
            "Black's rook on d8 has two jobs: it guards the queen on d7 and it guards the back \
             rank. A piece with two jobs can only do one of them. Make it choose: whichever it \
             keeps, you win the other.",
            "3r2k1/pp1q1ppp/8/8/6Q1/8/PP3PPP/4R1K1 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 6 },
            2,
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
        drill(
            "spare-tempo",
            "Opposition: the spare move",
            "Win the opposition with a pawn move in hand.",
            "This is the last drill's draw with the pawn one square further back, and that \
             changes everything. The kings face each other and you are to move, so Black has \
             the opposition; but a pawn move is a waiting move. Push the pawn one square, the \
             opposition is yours, and the king walks in front of the pawn as before.",
            "8/8/4k3/8/4K3/8/4P3/8 w - - 0 1",
            Color::White,
            Goal::Promote,
            15,
        ),
        drill(
            "rook-against-pawn",
            "Rook against pawn",
            "Stop a pawn its king is escorting.",
            "A rook alone can't stop a pawn that its king walks home: in the end the rook has to \
             give itself up for the pawn. Bring your king back first, towards the queening \
             square, then use the rook to keep Black's king away, and win the pawn while keeping \
             the rook.",
            "8/8/8/8/3k4/8/3p4/R6K w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 1 },
            12,
        ),
        drill(
            "philidor",
            "Philidor's defence",
            "Hold rook and pawn against rook.",
            "Keep your king in front of the pawn and your rook on the sixth rank, so White's king \
             can't come forward. Wait along that rank. The moment the pawn steps onto it, the \
             king has lost its shelter: drop the rook to the far end of the board and check \
             from behind. Lose the rook, or let the pawn through, and the drill is lost.",
            "4k3/R7/1r6/3KP3/8/8/8/8 w - - 0 1",
            Color::Black,
            Goal::Draw,
            20,
        ),
        drill(
            "lucena",
            "Lucena: build a bridge",
            "Promote with rook and pawn against rook.",
            "Your king is in front of its pawn and has to come out without being checked for \
             ever. First drive Black's king a file further away with a rook check. Then lift \
             your rook to the fourth rank, walk the king out, and block the last check with the \
             rook: that's the bridge. Promoting wins, and so does Black giving up the rook for \
             the pawn; either way you end at least four points up.",
            "1K6/1P1k4/8/8/8/8/r7/2R5 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 4 },
            15,
        ),
    ]
}

pub fn find(id: &str) -> Option<Drill> {
    drills().into_iter().find(|d| d.id == id)
}

fn moves_text(n: u32) -> String {
    format!("{n} {}", if n == 1 { "move" } else { "moves" })
}

fn points_text(n: i64) -> String {
    format!("{n} {}", if n == 1 { "point" } else { "points" })
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

/// `side`'s material minus the other side's.
fn balance(pos: &Chess, side: Color) -> i64 {
    let [white, black] = material_of(pos.board()).map(i64::from);
    match side {
        Color::White => white - black,
        Color::Black => black - white,
    }
}

/// How many of `side`'s knights and bishops still stand where they started.
fn minors_at_home(pos: &Chess, side: Color) -> u32 {
    let board = pos.board();
    let back = side.fold_wb(shakmaty::Rank::First, shakmaty::Rank::Eighth);
    let minors = board.by_color(side) & (board.knights() | board.bishops());
    minors
        .into_iter()
        .filter(|&sq| {
            sq.rank() == back
                && board
                    .role_at(sq)
                    .is_some_and(|role: Role| home_square(role, sq))
        })
        .count() as u32
}

/// Judge the drill after `moves` (UCI, both sides, from the drill's FEN).
pub fn assess(drill: &Drill, moves: &[&str]) -> Result<Status, GameError> {
    let student = drill.student;
    let mut game = Game::from_fen(&drill.fen)?;
    let start = balance(game.position(), student);
    let mut student_moves = 0;
    let (mut student_promoted, mut opponent_promoted) = (false, false);
    let (mut castled, mut lost_material) = (false, false);
    for uci in moves {
        let mover = game.turn();
        let played = game.play_uci(uci)?;
        let promoted = played.mv.promotion().is_some();
        if mover == student {
            student_moves += 1;
            student_promoted |= promoted;
            castled |= played.mv.is_castle();
            // Down on the start even after the student's own move: whatever
            // was taken wasn't taken back.
            lost_material |= balance(game.position(), student) < start;
        } else {
            opponent_promoted |= promoted;
        }
    }
    let status = game.final_status();
    let budget_spent = student_moves >= drill.moves;
    // Material the student won counts once the engine has had its answer,
    // so the last move of a budget is judged after the engine's reply.
    let to_move = game.turn() == student;
    let gained = balance(game.position(), student) - start;
    let lost = |reason: String| Ok(Status::Lost { reason });
    let won = |reason: String| Ok(Status::Won { reason });
    match drill.goal {
        Goal::Checkmate | Goal::Promote => {
            if let GameStatus::Checkmate { winner } = status {
                return if winner == student {
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
                && winner != student
            {
                return lost("Checkmated.".into());
            }
            if opponent_promoted {
                return lost("The pawn promoted.".into());
            }
            if lost_material {
                return lost("You lost material and couldn't win it back.".into());
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
        Goal::WinMaterial { points } => {
            if let GameStatus::Checkmate { winner } = status {
                return if winner == student {
                    won("Checkmate!".into())
                } else {
                    lost("You were checkmated.".into())
                };
            }
            if let Some(why) = draw_reason(status) {
                return lost(format!("A draw by {why}: the win slipped away."));
            }
            if to_move && gained >= i64::from(points) {
                return won(format!(
                    "You're {} up on the start: a won game.",
                    points_text(gained)
                ));
            }
            if budget_spent {
                if !to_move && student_moves == drill.moves {
                    return Ok(Status::Going { moves_left: 0 });
                }
                return lost(format!(
                    "Out of moves: the goal was to win {} of material within {}.",
                    points_text(points.into()),
                    moves_text(drill.moves)
                ));
            }
        }
        Goal::Develop => {
            if let GameStatus::Checkmate { winner } = status {
                return if winner == student {
                    won("Checkmate!".into())
                } else {
                    lost("You were checkmated.".into())
                };
            }
            if let Some(why) = draw_reason(status) {
                return lost(format!("A draw by {why} before you were developed."));
            }
            let at_home = minors_at_home(game.position(), student);
            if to_move && castled && at_home == 0 && gained >= 0 {
                return won("Castled, every minor piece out, and no material lost.".into());
            }
            if budget_spent {
                if !to_move && student_moves == drill.moves {
                    return Ok(Status::Going { moves_left: 0 });
                }
                let mut missing = Vec::new();
                if !castled {
                    missing.push("you haven't castled".to_string());
                }
                if at_home > 0 {
                    missing.push(format!(
                        "{at_home} {} still at home",
                        if at_home == 1 {
                            "minor piece is"
                        } else {
                            "minor pieces are"
                        }
                    ));
                }
                if gained < 0 {
                    missing.push(format!("you're {} down", points_text(-gained)));
                }
                return lost(format!("Out of moves: {}.", and_list(&missing)));
            }
        }
    }
    Ok(Status::Going {
        moves_left: drill.moves - student_moves,
    })
}

/// "a", "a and b", "a, b and c".
fn and_list(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(fen: &str, student: Color, goal: Goal, moves: u32) -> Drill {
        drill("t", "t", "t", "t", fen, student, goal, moves)
    }

    fn going(n: u32) -> Status {
        Status::Going { moves_left: n }
    }

    fn is_won(s: Status) -> bool {
        matches!(s, Status::Won { .. })
    }

    /// Play `line` (space-separated UCI) in drill `id`.
    fn play(id: &str, line: &str) -> Status {
        let d = find(id).unwrap_or_else(|| panic!("no drill {id}"));
        let moves: Vec<_> = line.split_whitespace().collect();
        assess(&d, &moves).unwrap_or_else(|e| panic!("{id}: {line}: {e}"))
    }

    #[test]
    fn every_drill_is_sound() {
        let all = drills();
        assert_eq!(all.len(), 18);
        let mut ids: Vec<_> = all.iter().map(|d| d.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len(), "ids are unique");
        for d in &all {
            let game = Game::from_fen(&d.fen).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            assert!(!game.final_status().is_game_over(), "{}", d.id);
            assert!(d.moves > 0, "{}", d.id);
            assert_eq!(assess(d, &[]).unwrap(), going(d.moves), "{}", d.id);
            assert_eq!(find(&d.id).as_ref(), Some(d));
            match d.goal {
                Goal::WinMaterial { points } => assert!(points > 0, "{}", d.id),
                // Castling has to be possible, and there has to be something to develop.
                Goal::Develop => {
                    assert!(game.position().castles().has_color(d.student), "{}", d.id);
                    assert!(minors_at_home(game.position(), d.student) > 0, "{}", d.id);
                }
                _ => {}
            }
        }
        // Mating drills start with the student to move; the defences start with the engine.
        assert_eq!(
            Game::from_fen(&find("queen-mate").unwrap().fen)
                .unwrap()
                .turn(),
            Color::White
        );
        for id in ["hold-the-draw", "philidor"] {
            let hold = find(id).unwrap();
            assert_ne!(Game::from_fen(&hold.fen).unwrap().turn(), hold.student);
        }
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
            "develop-and-castle",
            "early-queen",
            "back-rank-combination",
            "knight-fork",
            "pin",
            "skewer",
            "discovered-attack",
            "deflection",
            "spare-tempo",
            "rook-against-pawn",
            "philidor",
            "lucena",
        ] {
            assert!(ids.iter().any(|i| i == id), "{id} was renamed or removed");
        }
    }

    /// One route through the material, not a shelf: the first mate, the
    /// opening, tactics, the basic mates, then the endings.
    #[test]
    fn the_drills_run_in_order() {
        let ids: Vec<_> = drills().into_iter().map(|d| d.id).collect();
        assert_eq!(
            ids,
            [
                "back-rank-mate",
                "develop-and-castle",
                "early-queen",
                "back-rank-combination",
                "knight-fork",
                "pin",
                "skewer",
                "discovered-attack",
                "deflection",
                "two-rooks",
                "queen-mate",
                "rook-mate",
                "king-in-front",
                "hold-the-draw",
                "spare-tempo",
                "rook-against-pawn",
                "philidor",
                "lucena",
            ]
        );
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
        assert_eq!(assess(&d, &["g1g2", "h8h7"]).unwrap(), going(2));
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
        assert_eq!(assess(&d, &["d7d6"]).unwrap(), going(4));
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
        assert_eq!(assess(&d, &held).unwrap(), going(19));
        let short = Drill { moves: 1, ..d };
        assert_eq!(
            assess(&short, &held).unwrap(),
            Status::Won {
                reason: "You held for 1 move: that's a draw.".into()
            }
        );
    }

    #[test]
    fn a_draw_drill_is_lost_with_material_not_won_back() {
        // A rook trade White starts is fine once Black takes back...
        let d = custom(
            "8/2k5/2r5/2R1P3/3K4/8/8/8 w - - 0 1",
            Color::Black,
            Goal::Draw,
            10,
        );
        assert_eq!(assess(&d, &["c5c6", "c7c6", "d4c4"]).unwrap(), going(9));
        // ...but a rook left to be taken is lost the moment Black moves without taking back.
        assert_eq!(assess(&d, &["d4e4", "c7d8", "c5c6"]).unwrap(), going(9));
        assert_eq!(
            assess(&d, &["d4e4", "c7d8", "c5c6", "d8e7"]).unwrap(),
            Status::Lost {
                reason: "You lost material and couldn't win it back.".into()
            }
        );
        // The Philidor drill itself: waiting along the sixth rank is fine.
        assert_eq!(play("philidor", "a7a8 e8e7 a8a7 e7e8"), going(18));
        assert_eq!(
            play("philidor", "a7a8 e8d7 a8b8 b6b8"),
            Status::Going { moves_left: 18 },
            "winning White's rook is only better"
        );
        assert!(matches!(
            play("philidor", "a7a8 e8e7 a8b8 b6b8"),
            Status::Going { .. }
        ));
    }

    #[test]
    fn material_is_won_once_the_engine_has_answered() {
        // A loose rook: Qxd5 counts after Black's reply.
        let d = custom(
            "4k3/8/8/3r4/8/8/8/3QK3 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 5 },
            1,
        );
        assert_eq!(assess(&d, &["d1d5"]).unwrap(), going(0));
        assert_eq!(
            assess(&d, &["d1d5", "e8f8"]).unwrap(),
            Status::Won {
                reason: "You're 5 points up on the start: a won game.".into()
            }
        );
        // Defended by a pawn: the queen for a rook is 4 points down, not 5 up.
        let d = custom(
            "4k3/4p3/2p5/3r4/8/8/8/3QK3 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 5 },
            2,
        );
        assert_eq!(assess(&d, &["d1d5", "c6d5"]).unwrap(), going(1));
        assert_eq!(
            assess(&d, &["d1d5", "c6d5", "e1e2", "e8d7"]).unwrap(),
            Status::Lost {
                reason: "Out of moves: the goal was to win 5 points of material within 2 moves."
                    .into()
            }
        );
        // Mate counts, and so does a draw against.
        let d = custom(
            "6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 9 },
            1,
        );
        assert!(is_won(assess(&d, &["a1a8"]).unwrap()));
        let d = custom(
            "7k/8/5K2/8/8/8/8/6Q1 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 1 },
            3,
        );
        assert_eq!(
            assess(&d, &["g1g6"]).unwrap(),
            Status::Lost {
                reason: "A draw by stalemate: the win slipped away.".into()
            }
        );
    }

    #[test]
    fn the_tactics_are_won_by_their_combination() {
        // Each first line is Stockfish's; each fails line is a natural try that doesn't work.
        assert_eq!(play("knight-fork", "d5e7 g8f8"), going(1));
        assert_eq!(play("knight-fork", "d5e7 g8f8 e7c6"), going(0));
        assert!(is_won(play("knight-fork", "d5e7 g8f8 e7c6 b7c6")));
        assert!(is_won(play("knight-fork", "d5e7 g8f8 e7c6 a8e8")));
        assert!(!is_won(play("knight-fork", "d5f6 d7f6 d1d8 a8d8")));

        assert!(is_won(play("pin", "f1c4 d5c4 b3c4 a7a5")));
        assert!(is_won(play("pin", "f1c4 d5c4 c1c4 a7a5")));
        assert!(is_won(play("pin", "f1c4 g8f8 c4d5 b8d8")));
        assert!(!is_won(play("pin", "c1c8 b8c8 f1c4 c8c4")));

        assert!(is_won(play("skewer", "e2f3 d5e5 f3a8 e5f4")));
        assert!(!is_won(play("skewer", "e2c4 d5c4 g2g3 a8a1")));

        assert!(is_won(play("discovered-attack", "d3h7 g8h7 d1d6 c8f5")));
        assert!(is_won(play("discovered-attack", "d3h7 g8h8 d1d6 f6h7")));
        // Any other bishop move lets the queen get away.
        assert!(!is_won(play("discovered-attack", "d3e4 d6d1 f1d1 f6e4")));

        // Take the queen: if the rook takes back, it has left the back rank.
        assert_eq!(play("deflection", "g4d7 d8d7"), going(1));
        assert_eq!(
            play("deflection", "g4d7 d8d7 e1e8"),
            Status::Won {
                reason: "Checkmate!".into()
            }
        );
        assert!(is_won(play("deflection", "g4d7 d8f8")));
        assert!(!is_won(play("deflection", "e1e8 d8e8 g4d7 e8e1")));

        assert_eq!(
            play("back-rank-combination", "e2e8 d7e8 e1e8"),
            Status::Won {
                reason: "Checkmate!".into()
            }
        );
        assert!(!is_won(play(
            "back-rank-combination",
            "e2e7 e8e7 e1e7 d7e7"
        )));
    }

    #[test]
    fn develop_drills_need_castling_every_minor_piece_and_level_material() {
        // Stockfish's own answers to a sound development.
        let italian = "e2e4 e7e5 g1f3 b8c6 f1c4 g8f6 d2d3 f8c5 e1g1 d7d6 b1c3 e8g8";
        assert_eq!(play("develop-and-castle", italian), going(4));
        assert_eq!(
            play("develop-and-castle", &format!("{italian} c1g5")),
            going(3),
            "judged once Black has answered"
        );
        assert_eq!(
            play("develop-and-castle", &format!("{italian} c1g5 h7h6")),
            Status::Won {
                reason: "Castled, every minor piece out, and no material lost.".into()
            }
        );
        // Castling and developing with a pawn gone isn't enough.
        let gambit = "e2e4 e7e5 g1f3 b8c6 f1c4 g8f6 e1g1 f6e4 d2d3 e4f6 b1c3 f8e7 c1g5 e8g8";
        assert_eq!(play("develop-and-castle", gambit), going(3));
        let spent = format!("{gambit} a2a3 d7d6 a3a4 a7a6 h2h3 h7h6");
        assert_eq!(
            play("develop-and-castle", &spent),
            Status::Lost {
                reason: "Out of moves: you're 1 point down.".into()
            }
        );
        // Nothing done at all.
        let shuffle = "a2a3 e7e5 a3a4 d7d5 h2h3 g8f6 h3h4 b8c6 a1a3 f8d6 a3a1 e8g8 \
                       a1a3 c8e6 a3a1 d8e7 a1a3 a8d8 a3a1 f8e8";
        assert_eq!(
            play("develop-and-castle", shuffle),
            Status::Lost {
                reason: "Out of moves: you haven't castled and 4 minor pieces are still at home."
                    .into()
            }
        );

        // The early queen: cover f7 and develop...
        assert_eq!(
            play(
                "early-queen",
                "g7g6 h5f3 g8f6 g1e2 f8g7 d2d3 e8g8 b1c3 d7d6 a2a3 c8e6 e1g1"
            ),
            Status::Won {
                reason: "Castled, every minor piece out, and no material lost.".into()
            }
        );
        // ...or be mated.
        assert_eq!(
            play("early-queen", "g8f6 h5f7"),
            Status::Lost {
                reason: "You were checkmated.".into()
            }
        );
    }

    #[test]
    fn the_endings_are_won_by_their_technique() {
        // The spare move: e3 hands Black the move; a king move lets Black keep the opposition.
        assert_eq!(play("spare-tempo", "e2e3 e6f6"), going(14));
        assert_eq!(
            play(
                "spare-tempo",
                "e2e3 e6f6 e4d5 f6e7 d5e5 e7d7 e5f6 d7c6 e3e4 c6d7 e4e5 d7e8 \
                 f6e6 e8d8 e6f7 d8c7 e5e6 c7b6 e6e7 b6c5 f7e6 c5c4 e7e8q"
            ),
            Status::Won {
                reason: "Promoted! The new queen wins easily.".into()
            }
        );

        // Rook against pawn: the king comes back, then the rook wins the pawn.
        let line = "h1g2 d4e3 g2f1 e3d3 f1f2 d3e4 f2e2 e4d4 a1d1 d4e4 d1d2";
        assert_eq!(play("rook-against-pawn", line), going(6));
        assert_eq!(
            play("rook-against-pawn", &format!("{line} e4f4")),
            Status::Won {
                reason: "You're 1 point up on the start: a won game.".into()
            }
        );
        // The rook alone ends up giving itself for the pawn.
        assert!(!is_won(play(
            "rook-against-pawn",
            "a1d1 d4e3 h1g2 e3e2 d1d2 e2d2"
        )));

        // Lucena: the bridge, then the pawn promotes.
        let bridge = "c1d1 d7e7 d1d4 a2a1 b8c7 a1c1 c7b6 c1b1 b6c6 e7e6 d4d6 e6f5 \
                      d6d5 f5e4 d5b5 b1c1 c6b6 c1f1 b7b8q";
        assert_eq!(play("lucena", bridge), going(5));
        assert!(is_won(play("lucena", &format!("{bridge} f1f6"))));
        // Queening into the rook's capture isn't a win...
        let d = custom(
            "8/1P6/3K4/8/8/5k2/8/1r6 w - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 4 },
            3,
        );
        assert_eq!(assess(&d, &["b7b8q", "b1b8"]).unwrap(), going(2));
        // ...but Black giving the rook for the pawn is.
        let d = custom(
            "1K6/1P6/8/8/8/5k2/8/1r5R b - - 0 1",
            Color::White,
            Goal::WinMaterial { points: 4 },
            3,
        );
        assert!(is_won(assess(&d, &["b1b7", "b8b7", "f3e3"]).unwrap()));
    }
}
