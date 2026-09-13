//! Talking to a UCI engine: parsing what it prints and presenting scores.
//!
//! The engine itself runs elsewhere (a WASM worker in the browser, a process
//! on desktop, a pool on the server). This module only knows the text
//! protocol, so it can be tested without an engine and reused everywhere.

use std::fmt;

use shakmaty::{Chess, Color, san::SanPlus, uci::UciMove};

use crate::game::write_movetext;

/// An evaluation as reported by the engine: from the point of view of the
/// side to move unless converted with [`Score::for_white`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Score {
    /// Centipawns.
    Cp(i32),
    /// Mate in this many moves; negative when the side being scored gets mated.
    Mate(i32),
}

impl Score {
    /// The same evaluation from the other side's point of view.
    pub fn flip(self) -> Score {
        match self {
            Score::Cp(cp) => Score::Cp(-cp),
            Score::Mate(n) => Score::Mate(-n),
        }
    }

    /// Convert an engine score (side-to-move perspective) for the position
    /// where `turn` is to move into White's perspective.
    pub fn for_white(self, turn: Color) -> Score {
        if turn.is_white() { self } else { self.flip() }
    }

    /// Winning chances for the side this score favours, from -1 (certain
    /// loss) to 1 (certain win). Uses Lichess's logistic mapping so an eval
    /// bar reads the same as theirs.
    pub fn win_chance(self) -> f64 {
        match self {
            Score::Cp(cp) => 2.0 / (1.0 + (-0.003_682_08 * f64::from(cp)).exp()) - 1.0,
            Score::Mate(n) if n > 0 => 1.0,
            Score::Mate(_) => -1.0,
        }
    }

    /// The share of an eval bar (0 to 1) to fill for the side this score favours.
    pub fn bar_fraction(self) -> f64 {
        (1.0 + self.win_chance()) / 2.0
    }
}

/// `+0.35`, `-1.20`, `#3`, `#-3`. Mate in 0 is shown as `#0` (the side is
/// already mated).
impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Score::Cp(cp) => write!(
                f,
                "{}{:.2}",
                if cp >= 0 { "+" } else { "-" },
                f64::from(cp.abs()) / 100.0
            ),
            Score::Mate(n) => write!(f, "#{n}"),
        }
    }
}

/// One principal variation from an `info` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub depth: u32,
    pub seldepth: Option<u32>,
    /// 1-based rank among the engine's `MultiPV` lines.
    pub multipv: u32,
    /// From the side to move's perspective.
    pub score: Score,
    pub nodes: Option<u64>,
    pub nps: Option<u64>,
    pub time_ms: Option<u64>,
    pub pv: Vec<UciMove>,
}

impl Line {
    pub fn best_move(&self) -> Option<UciMove> {
        self.pv.first().copied()
    }
}

/// A line of engine output, classified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    UciOk,
    ReadyOk,
    /// A complete principal variation. Partial `info` lines (no `pv`, or a
    /// `lowerbound`/`upperbound` score) are reported as [`Message::Other`].
    Info(Line),
    /// `best` is `None` when the engine has no move (`bestmove (none)`).
    BestMove {
        best: Option<UciMove>,
        ponder: Option<UciMove>,
    },
    Other(String),
}

/// Classify one line of engine output.
pub fn parse_message(line: &str) -> Message {
    let line = line.trim();
    let mut words = line.split_whitespace();
    match words.next() {
        Some("uciok") => Message::UciOk,
        Some("readyok") => Message::ReadyOk,
        Some("bestmove") => Message::BestMove {
            best: words.next().and_then(|w| w.parse().ok()),
            ponder: match words.next() {
                Some("ponder") => words.next().and_then(|w| w.parse().ok()),
                _ => None,
            },
        },
        Some("info") => {
            parse_info(line).map_or_else(|| Message::Other(line.to_string()), Message::Info)
        }
        _ => Message::Other(line.to_string()),
    }
}

fn parse_info(line: &str) -> Option<Line> {
    let mut depth = None;
    let mut seldepth = None;
    let mut multipv = 1;
    let mut score = None;
    let mut nodes = None;
    let mut nps = None;
    let mut time_ms = None;
    let mut pv = Vec::new();

    let mut words = line.split_whitespace().skip(1).peekable();
    while let Some(key) = words.next() {
        match key {
            "depth" => depth = words.next()?.parse().ok(),
            "seldepth" => seldepth = words.next()?.parse().ok(),
            "multipv" => multipv = words.next()?.parse().ok()?,
            "nodes" => nodes = words.next()?.parse().ok(),
            "nps" => nps = words.next()?.parse().ok(),
            "time" => time_ms = words.next()?.parse().ok(),
            "score" => {
                let kind = words.next()?;
                let value: i32 = words.next()?.parse().ok()?;
                score = Some(match kind {
                    "cp" => Score::Cp(value),
                    "mate" => Score::Mate(value),
                    _ => return None,
                });
                if matches!(words.peek(), Some(&"lowerbound" | &"upperbound")) {
                    return None;
                }
            }
            "pv" => {
                pv = words.by_ref().map_while(|w| w.parse().ok()).collect();
                break;
            }
            "string" => return None,
            // Keys with a single value we don't use.
            "currmove" | "currmovenumber" | "hashfull" | "tbhits" | "sbhits" | "cpuload"
            | "multipv_" => {
                words.next();
            }
            _ => {}
        }
    }

    (!pv.is_empty()).then_some(Line {
        depth: depth?,
        seldepth,
        multipv,
        score: score?,
        nodes,
        nps,
        time_ms,
        pv,
    })
}

/// Render a principal variation in SAN, stopping at the first move that isn't
/// legal (engines can print truncated or hash-corrupted tails).
pub fn pv_san(pos: &Chess, pv: &[UciMove]) -> Vec<SanPlus> {
    let mut pos = pos.clone();
    let mut out = Vec::with_capacity(pv.len());
    for uci in pv {
        let Ok(mv) = uci.to_move(&pos) else { break };
        out.push(SanPlus::from_move_and_play_unchecked(&mut pos, mv));
    }
    out
}

/// A principal variation as numbered movetext from `pos`, e.g. `12... Nf6 13. Bd3`.
pub fn pv_movetext(pos: &Chess, pv: &[UciMove]) -> String {
    write_movetext(pos, pv_san(pos, pv).iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uci(s: &str) -> UciMove {
        s.parse().unwrap()
    }

    #[test]
    fn parses_a_full_info_line() {
        let msg = parse_message(
            "info depth 18 seldepth 25 multipv 2 score cp -35 nodes 1234567 nps 800000 hashfull 120 tbhits 0 time 1543 pv e7e5 g1f3 b8c6",
        );
        assert_eq!(
            msg,
            Message::Info(Line {
                depth: 18,
                seldepth: Some(25),
                multipv: 2,
                score: Score::Cp(-35),
                nodes: Some(1_234_567),
                nps: Some(800_000),
                time_ms: Some(1543),
                pv: vec![uci("e7e5"), uci("g1f3"), uci("b8c6")],
            })
        );
    }

    #[test]
    fn mate_scores_and_defaults() {
        let Message::Info(line) = parse_message("info depth 5 score mate -2 pv h7h6") else {
            panic!()
        };
        assert_eq!(line.score, Score::Mate(-2));
        assert_eq!(line.multipv, 1);
        assert_eq!(line.best_move(), Some(uci("h7h6")));
    }

    #[test]
    fn partial_info_lines_are_not_variations() {
        assert!(matches!(
            parse_message("info depth 12 currmove e2e4 currmovenumber 1"),
            Message::Other(_)
        ));
        assert!(matches!(
            parse_message("info depth 12 score cp 20 lowerbound nodes 100 pv e2e4"),
            Message::Other(_)
        ));
        assert!(matches!(
            parse_message("info string NNUE evaluation using nn-abc.nnue"),
            Message::Other(_)
        ));
        assert!(matches!(
            parse_message("info depth 3 pv e2e4"),
            Message::Other(_)
        ));
    }

    #[test]
    fn handshake_and_bestmove() {
        assert_eq!(parse_message("uciok"), Message::UciOk);
        assert_eq!(parse_message("readyok\n"), Message::ReadyOk);
        assert_eq!(
            parse_message("bestmove e2e4 ponder e7e5"),
            Message::BestMove {
                best: Some(uci("e2e4")),
                ponder: Some(uci("e7e5"))
            }
        );
        assert_eq!(
            parse_message("bestmove (none)"),
            Message::BestMove {
                best: None,
                ponder: None
            }
        );
        assert_eq!(
            parse_message("id name Stockfish 18"),
            Message::Other("id name Stockfish 18".into())
        );
    }

    #[test]
    fn score_perspective_and_display() {
        assert_eq!(Score::Cp(35).for_white(Color::White), Score::Cp(35));
        assert_eq!(Score::Cp(35).for_white(Color::Black), Score::Cp(-35));
        assert_eq!(Score::Mate(3).for_white(Color::Black), Score::Mate(-3));
        assert_eq!(Score::Cp(35).to_string(), "+0.35");
        assert_eq!(Score::Cp(-120).to_string(), "-1.20");
        assert_eq!(Score::Cp(0).to_string(), "+0.00");
        assert_eq!(Score::Mate(3).to_string(), "#3");
        assert_eq!(Score::Mate(-1).to_string(), "#-1");
    }

    #[test]
    fn win_chance_is_symmetric_and_bounded() {
        assert!((Score::Cp(0).win_chance()).abs() < 1e-9);
        assert!((Score::Cp(100).win_chance() + Score::Cp(-100).win_chance()).abs() < 1e-9);
        assert!(Score::Cp(100).win_chance() > 0.15 && Score::Cp(100).win_chance() < 0.25);
        assert!(Score::Cp(10_000).win_chance() > 0.99);
        assert_eq!(Score::Mate(1).win_chance(), 1.0);
        assert_eq!(Score::Mate(-4).win_chance(), -1.0);
        assert_eq!(Score::Cp(0).bar_fraction(), 0.5);
        assert_eq!(Score::Mate(-4).bar_fraction(), 0.0);
    }

    #[test]
    fn pv_to_san_stops_at_illegal_moves() {
        let pos = Chess::default();
        let pv = [uci("e2e4"), uci("e7e5"), uci("g1f3"), uci("a1a8")];
        let san: Vec<String> = pv_san(&pos, &pv).iter().map(ToString::to_string).collect();
        assert_eq!(san, ["e4", "e5", "Nf3"]);
        assert_eq!(pv_movetext(&pos, &pv), "1. e4 e5 2. Nf3");

        let pos: Chess =
            crate::Game::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1")
                .unwrap()
                .position()
                .clone();
        assert_eq!(
            pv_movetext(&pos, &[uci("e7e5"), uci("g1f3")]),
            "1... e5 2. Nf3"
        );
        assert_eq!(pv_movetext(&pos, &[]), "");
    }
}
