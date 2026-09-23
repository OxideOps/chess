//! Game review: the moves of a finished game that changed it most.
//!
//! The engine runs elsewhere (in the browser); it evaluates every position
//! of the game and hands the results here. A move's swing is how much of
//! [`Score::win_chance`] it gave away, judged the same way as a drill
//! mistake ([`Score::is_mistake`]). Moves made after the game was already
//! decided don't count: a blunder in a lost position moves the number a lot
//! and teaches nothing.

use shakmaty::{Chess, Color, Position, san::SanPlus, uci::UciMove};

use crate::{
    Game,
    engine::{Score, pv_movetext, pv_san},
};

/// How many swings a review shows.
pub const SWINGS: usize = 3;

/// Winning chances (in [`Score::win_chance`]'s -1 to 1) beyond which a game
/// counts as decided: about six pawns. A move made from a position already
/// this lost isn't a swing, however much it moves the number.
///
/// Only the losing end needs the filter: a player this far *ahead* can only
/// make [`Score::is_mistake`] by throwing a real part of the win away, and
/// that is worth seeing. (One who stays this far ahead can't drop enough to
/// be a mistake at all.)
pub const DECIDED: f64 = 0.8;

/// What the engine made of one position: its score for the side to move and
/// the line it expects, best move first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub score: Score,
    pub pv: Vec<UciMove>,
}

/// One move that threw away winning chances.
#[derive(Debug, Clone, PartialEq)]
pub struct Swing {
    /// The ply the move leads to (1 for the game's first move).
    pub ply: usize,
    pub mover: Color,
    /// The position the move was played from.
    pub before_position: Chess,
    /// The move played.
    pub played: SanPlus,
    pub played_uci: UciMove,
    /// The evaluation before and after the move, from the mover's point of view.
    pub before: Score,
    pub after: Score,
    /// Winning chances given away: `before.win_chance() - after.win_chance()`.
    pub drop: f64,
    /// The engine's line from `before_position`, legal moves only.
    pub best: Vec<UciMove>,
}

impl Swing {
    /// The move played, numbered: `12... Qxd4`.
    pub fn played_movetext(&self) -> String {
        pv_movetext(&self.before_position, &[self.played_uci])
    }

    /// The engine's line, numbered: `12... Nf6 13. Bd3`.
    pub fn best_movetext(&self) -> String {
        pv_movetext(&self.before_position, &self.best)
    }
}

/// The score of the position after `ply` plies of `game`'s current line, for
/// the side to move there: the engine's, or the result when the game is over
/// there (the engine has nothing to search in a finished position).
fn score_at(game: &Game, ply: usize, evals: &[Option<Evaluation>]) -> Option<Score> {
    let mut at = game.clone();
    at.go_to_ply(ply);
    let status = at.status();
    if status.is_game_over() {
        // A mated side to move has lost; every other ending is a draw.
        return Some(if status.winner().is_some() {
            Score::Mate(0)
        } else {
            Score::Cp(0)
        });
    }
    evals.get(ply)?.as_ref().map(|e| e.score)
}

/// The moves of `game`'s current line that gave away the most winning
/// chances, biggest first, at most `count` of them.
///
/// `evals[i]` is the engine's view of the position after `i` plies (so
/// `evals[0]` is the start); `None` where it wasn't searched. A move is a
/// candidate when both the position before it and after it have a score,
/// it isn't the engine's own first choice, the mover wasn't already lost
/// ([`DECIDED`]), and it is a mistake by [`Score::is_mistake`]. `side`
/// keeps only one player's moves.
pub fn swings(
    game: &Game,
    evals: &[Option<Evaluation>],
    side: Option<Color>,
    count: usize,
) -> Vec<Swing> {
    let mut position = game.start_position().clone();
    let mut found = Vec::new();
    let mut before_score = score_at(game, 0, evals);
    for (i, played) in game.moves().iter().enumerate() {
        let ply = i + 1;
        let mover = position.turn();
        let after_score = score_at(game, ply, evals);
        let best = evals
            .get(i)
            .and_then(Option::as_ref)
            .map(|e| legal_prefix(&position, &e.pv))
            .unwrap_or_default();
        if let (Some(before), Some(after)) = (before_score, after_score.map(for_mover))
            && side.is_none_or(|s| s == mover)
            && best.first().is_some_and(|b| *b != played.uci)
            && before.win_chance() > -DECIDED
            && Score::is_mistake(before, after)
        {
            found.push(Swing {
                ply,
                mover,
                before_position: position.clone(),
                played: played.san,
                played_uci: played.uci,
                before,
                after,
                drop: before.win_chance() - after.win_chance(),
                best,
            });
        }
        position.play_unchecked(played.mv);
        before_score = after_score;
    }
    // Biggest first; the earlier move wins a tie.
    found.sort_by(|a, b| b.drop.total_cmp(&a.drop).then(a.ply.cmp(&b.ply)));
    found.truncate(count);
    found
}

/// A score for the side to move after a move, turned round for the side that
/// made it. `Mate(0)` (the side to move is mated) has no sign to flip, so it
/// becomes a win for the mover outright.
fn for_mover(after: Score) -> Score {
    match after {
        Score::Mate(0) => Score::Mate(1),
        other => other.flip(),
    }
}

/// The moves of `pv` up to the first one that isn't legal (engines can print
/// truncated or hash-corrupted tails).
fn legal_prefix(position: &Chess, pv: &[UciMove]) -> Vec<UciMove> {
    pv[..pv_san(position, pv).len()].to_vec()
}

/// `game` with each swing's better line added as a variation where the move
/// was played, the cursor back at the start of the main line: something the
/// analysis board can open.
pub fn with_lines(game: &Game, swings: &[Swing]) -> Game {
    let mut out = game.clone();
    let main = game.line().to_vec();
    for swing in swings {
        out.go_to_node(main[swing.ply - 1]);
        for mv in &swing.best {
            if out.play_here_uci(&mv.to_string()).is_err() {
                break;
            }
        }
    }
    out.go_to_node(main[0]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uci(s: &str) -> UciMove {
        s.parse().unwrap()
    }

    fn eval(cp: i32, pv: &[&str]) -> Option<Evaluation> {
        Some(Evaluation {
            score: Score::Cp(cp),
            pv: pv.iter().map(|m| uci(m)).collect(),
        })
    }

    /// Scholar's mate, with Black's losing 3... Nf6?? and White's mate.
    fn scholars_mate() -> (Game, Vec<Option<Evaluation>>) {
        let game = Game::from_pgn("1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7#").unwrap();
        let evals = vec![
            eval(30, &["e2e4"]),          // start, White to move
            eval(-30, &["e7e5"]),         // after 1. e4, Black
            eval(30, &["g1f3"]),          // after 1... e5, White
            eval(-10, &["b8c6"]),         // after 2. Qh5, Black
            eval(40, &["f1c4"]),          // after 2... Nc6, White
            eval(-60, &["g7g6", "h5f3"]), // after 3. Bc4, Black: g6 holds
            Some(Evaluation {
                score: Score::Mate(1),
                pv: vec![uci("h5f7")],
            }), // after 3... Nf6, White mates
            None,                         // checkmate: never searched
        ];
        (game, evals)
    }

    #[test]
    fn finds_the_move_that_lost_the_game() {
        let (game, evals) = scholars_mate();
        let found = swings(&game, &evals, None, SWINGS);
        assert_eq!(found.len(), 1);
        let s = &found[0];
        assert_eq!(s.ply, 6);
        assert_eq!(s.mover, Color::Black);
        assert_eq!(s.played.to_string(), "Nf6");
        assert_eq!(s.played_movetext(), "3... Nf6");
        assert_eq!(s.before, Score::Cp(-60));
        assert_eq!(s.after, Score::Mate(-1));
        assert_eq!(s.best_movetext(), "3... g6 4. Qf3");
        assert!(s.drop > 0.7);
    }

    #[test]
    fn a_mate_the_engine_did_not_pick_is_no_mistake() {
        let (game, mut evals) = scholars_mate();
        // Say the engine had preferred a slower win: Qxf7# still wins outright.
        evals[6] = Some(Evaluation {
            score: Score::Mate(2),
            pv: vec![uci("c4f7")],
        });
        let found = swings(&game, &evals, None, SWINGS);
        assert_eq!(found.iter().map(|s| s.ply).collect::<Vec<_>>(), [6]);
    }

    #[test]
    fn keeps_to_one_side_when_asked() {
        let (game, evals) = scholars_mate();
        assert!(swings(&game, &evals, Some(Color::White), SWINGS).is_empty());
        assert_eq!(swings(&game, &evals, Some(Color::Black), SWINGS).len(), 1);
    }

    #[test]
    fn the_engines_own_move_is_never_a_swing() {
        let (game, mut evals) = scholars_mate();
        // The engine at a shallow depth liked Nf6 too: then it isn't a swing,
        // however the score moved.
        evals[5] = eval(-60, &["g8f6"]);
        assert!(swings(&game, &evals, None, SWINGS).is_empty());
    }

    #[test]
    fn unsearched_positions_are_skipped() {
        let (game, mut evals) = scholars_mate();
        evals[5] = None;
        assert!(swings(&game, &evals, None, SWINGS).is_empty());
        // A review cut short has fewer evaluations than positions.
        let (game, evals) = scholars_mate();
        assert!(swings(&game, &evals[..5], None, SWINGS).is_empty());
    }

    #[test]
    fn a_blunder_in_a_lost_position_does_not_count() {
        let (game, mut evals) = scholars_mate();
        // Black was already lost before 3... Nf6: nothing left to throw away.
        evals[5] = eval(-900, &["g7g6"]);
        assert!(swings(&game, &evals, None, SWINGS).is_empty());
    }

    #[test]
    fn throwing_away_a_won_game_counts() {
        // White had mate in four and let it go to a level game.
        let game = Game::from_pgn("1. e4 e5").unwrap();
        let evals = vec![
            Some(Evaluation {
                score: Score::Mate(4),
                pv: vec![uci("d2d4")],
            }),
            eval(0, &["g1f3"]),
            eval(0, &["d2d4"]),
        ];
        let found = swings(&game, &evals, None, SWINGS);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].ply, 1);
    }

    #[test]
    fn biggest_first_and_capped() {
        let game = Game::from_pgn("1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6").unwrap();
        // Made-up scores, each for the side to move. A move is a drop for its
        // mover when the scores either side of it are both good for the side
        // to move: the mover was better before, the opponent is after.
        let evals = vec![
            eval(0, &["d2d4"]),
            eval(100, &["c7c5"]), // 1. e4: 0 -> -100, too small to be a mistake
            eval(200, &["d2d4"]), // 1... e5: +100 -> -200
            eval(400, &["d7d6"]), // 2. Nf3: +200 -> -400, the biggest
            eval(0, &["d2d4"]),   // 2... Nc6: +400 -> 0
            eval(-10, &["g8f6"]), // 3. Bb5: nothing
            eval(0, &["b5c6"]),   // 3... a6: nothing
            eval(500, &["d7d6"]), // 4. Ba4: 0 -> -500
            eval(100, &["e1g1"]), // 4... Nf6: +500 -> -100
        ];
        let all = swings(&game, &evals, None, 10);
        let plies: Vec<usize> = all.iter().map(|s| s.ply).collect();
        assert_eq!(plies, [3, 8, 7, 4, 2]);
        assert!(all.windows(2).all(|w| w[0].drop >= w[1].drop));
        let top: Vec<usize> = swings(&game, &evals, None, SWINGS)
            .iter()
            .map(|s| s.ply)
            .collect();
        assert_eq!(top, [3, 8, 7]);
    }

    #[test]
    fn better_lines_become_variations() {
        let (game, evals) = scholars_mate();
        let found = swings(&game, &evals, None, SWINGS);
        let with = with_lines(&game, &found);
        assert_eq!(
            with.movetext(),
            "1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 (3... g6 4. Qf3) 4. Qxf7#"
        );
        assert_eq!(with.cursor(), 0);
        assert_eq!(with.ply_count(), 7);
    }

    #[test]
    fn illegal_tails_are_dropped_from_the_line() {
        let (game, mut evals) = scholars_mate();
        evals[5] = eval(-60, &["g7g6", "h5f3", "a1a8"]);
        let found = swings(&game, &evals, None, SWINGS);
        assert_eq!(found[0].best, [uci("g7g6"), uci("h5f3")]);
    }
}
