//! Build the coach's eval cases from a local Stockfish.
//!
//! ```sh
//! cargo run -p server --example coach_cases > crates/server/tests/fixtures/coach_eval.json
//! STOCKFISH=/path/to/stockfish DEPTH=22 cargo run -p server --example coach_cases
//! ```
//!
//! The positions live in `CASES` below: opening and middlegame positions,
//! tactics taken from the puzzle fixture, endgame studies, and mistakes (a
//! drill blunder, or the move `Played::Worst` finds to be the worst legal
//! one). For each, Stockfish's lines go into the case exactly as the client
//! would send them, so `coach_eval` needs no engine. Rebuild the fixture when
//! the cases change; the numbers shift a little between Stockfish versions,
//! which is why the file is committed rather than generated on the fly.

use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use chess_core::{
    engine::{Line, Message, Score, parse_message},
    shakmaty::{
        CastlingMode, Chess, EnPassantMode, Position,
        fen::Fen,
        san::{San, SanPlus},
        uci::UciMove,
    },
};
use serde_json::{Value, json};

/// Where a case's position comes from.
enum Start {
    /// Moves from the initial position, in SAN.
    Moves(&'static str),
    /// A FEN, then moves in UCI (a puzzle's setup move, say).
    Fen(&'static str, &'static str),
}

/// The move a mistake case plays.
enum Played {
    Uci(&'static str),
    /// The legal move that throws the most away, found by a shallow search:
    /// only for positions with few moves.
    Worst,
}

enum Ask {
    Explain,
    Mistake {
        played: Played,
        drill: Option<&'static str>,
    },
}

struct Case {
    name: &'static str,
    start: Start,
    ask: Ask,
    /// Questions asked after the answer, with the move (SAN) to give
    /// Stockfish's line for, if the question names one.
    follow_ups: &'static [(&'static str, Option<&'static str>)],
}

const EXPLAIN: Ask = Ask::Explain;

const fn drill(played: &'static str, drill: &'static str) -> Ask {
    Ask::Mistake {
        played: Played::Uci(played),
        drill: Some(drill),
    }
}

const CASES: &[Case] = &[
    // --- openings and middlegames -------------------------------------
    Case {
        name: "scholars-mate",
        start: Start::Fen(
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
            "",
        ),
        ask: EXPLAIN,
        follow_ups: &[(
            "Why can't Black just take my queen with the knight first?",
            None,
        )],
    },
    Case {
        name: "after-e4",
        start: Start::Moves("e4"),
        ask: EXPLAIN,
        follow_ups: &[
            ("Why not 1... d5 instead?", Some("d5")),
            ("Which is better for a beginner?", None),
        ],
    },
    Case {
        name: "opera-game-queen-sacrifice",
        start: Start::Moves(
            "e4 e5 Nf3 d6 d4 Bg4 dxe5 Bxf3 Qxf3 dxe5 Bc4 Nf6 Qb3 Qe7 Nc3 c6 Bg5 b5 Nxb5 cxb5 \
             Bxb5+ Nbd7 O-O-O Rd8 Rxd7 Rxd7 Rd1 Qe6 Bxd7+ Nxd7",
        ),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "legal-trap",
        start: Start::Moves("e4 e5 Nf3 d6 Bc4 Bg4 Nc3 g6"),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "queens-gambit-pin",
        start: Start::Moves("d4 d5 c4 e6 Nc3 Nf6 Bg5"),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "italian-center",
        start: Start::Moves("e4 e5 Nf3 Nc6 Bc4 Bc5 c3 Nf6 d4 exd4 cxd4 Bb4+ Bd2"),
        ask: EXPLAIN,
        follow_ups: &[("Why not castle with 7... O-O?", Some("O-O"))],
    },
    Case {
        name: "french-advance",
        start: Start::Moves("e4 e6 d4 d5 e5 c5 c3 Nc6 Nf3 Qb6"),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "sicilian-najdorf",
        start: Start::Moves("e4 c5 Nf3 d6 d4 cxd4 Nxd4 Nf6 Nc3 a6"),
        ask: EXPLAIN,
        follow_ups: &[("What is the point of a6 for Black?", None)],
    },
    Case {
        name: "kings-indian-closed",
        start: Start::Moves("d4 Nf6 c4 g6 Nc3 Bg7 e4 d6 Nf3 O-O Be2 e5 O-O Nc6 d5 Ne7"),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "isolated-queen-pawn",
        start: Start::Moves(
            "d4 d5 c4 e6 Nc3 Nf6 cxd5 exd5 Bg5 Be7 e3 O-O Bd3 Nbd7 Nf3 Re8 O-O Nf8",
        ),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "two-knights-fried-liver",
        start: Start::Moves("e4 e5 Nf3 Nc6 Bc4 Nf6 Ng5 d5 exd5 Nxd5"),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    // --- tactics, from the puzzle fixture -----------------------------
    Case {
        name: "tactic-mate-in-two",
        start: Start::Fen(
            "5r1k/6p1/1Q2pq1p/P4r2/3P4/2P1R3/6PP/4R1K1 w - - 1 39",
            "d4d5",
        ),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "tactic-anastasia-mate",
        start: Start::Fen(
            "r4r1k/1pp1NppR/3p4/p3n3/4P3/1PPP2P1/1P1K2P1/3R4 b - - 0 22",
            "h8h7",
        ),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "tactic-discovered-attack",
        start: Start::Fen("7r/ppp1Nk1p/2n5/8/8/2P2pP1/P6P/R1Br1BK1 b - - 0 20", "f7e7"),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "tactic-fork",
        start: Start::Fen(
            "2r5/3Qnk1p/8/4B2b/Pp2p3/1P2P3/5PPP/3R2K1 w - - 3 32",
            "d1d6",
        ),
        ask: EXPLAIN,
        follow_ups: &[("Why can't White just take my rook?", None)],
    },
    Case {
        name: "tactic-hanging-piece",
        start: Start::Fen("3r3r/pQNk1ppp/1qnR1n2/1B6/8/8/PPP3PP/5R1K b - - 0 19", ""),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "tactic-sacrifice",
        start: Start::Fen(
            "k1r1b3/p1r1nppp/Bp1qpn2/2Np4/1P1P4/PQR1PN2/5PPP/2R3K1 b - - 1 19",
            "",
        ),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    // --- endgames -----------------------------------------------------
    Case {
        name: "lucena",
        start: Start::Fen("1K1k4/1P6/8/8/8/8/r7/2R5 w - - 0 1", ""),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "philidor-draw",
        start: Start::Fen("3k4/8/3K4/3P4/8/8/6r1/5R2 b - - 0 1", ""),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "rook-vs-pawn",
        start: Start::Fen("8/8/8/8/8/3k4/3p4/3K1R2 w - - 0 1", ""),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "opposite-bishops",
        start: Start::Fen("8/8/4k3/2b5/8/2B5/4P3/4K3 w - - 0 1", ""),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    Case {
        name: "king-in-front",
        start: Start::Fen("4k3/8/4K3/4P3/8/8/8/8 w - - 0 1", ""),
        ask: EXPLAIN,
        follow_ups: &[],
    },
    // --- mistakes -----------------------------------------------------
    Case {
        name: "queen-mate-hangs-the-queen",
        start: Start::Fen("8/8/8/3k4/8/8/7Q/4K3 w - - 0 1", ""),
        ask: drill("h2e5", "queen-mate"),
        follow_ups: &[("What if I had played Qd6+ instead?", Some("Qd6+"))],
    },
    Case {
        name: "queen-mate-stalemate",
        start: Start::Fen("k7/8/1Q6/8/8/8/8/4K3 w - - 0 1", ""),
        ask: drill("b6c7", "queen-mate"),
        follow_ups: &[],
    },
    Case {
        name: "back-rank-mate-missed",
        start: Start::Fen("6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1", ""),
        ask: drill("a1a7", "back-rank-mate"),
        follow_ups: &[],
    },
    Case {
        name: "king-in-front-gives-up-the-win",
        start: Start::Fen("4k3/8/4K3/4P3/8/8/8/8 w - - 0 1", ""),
        ask: Ask::Mistake {
            played: Played::Worst,
            drill: Some("king-in-front"),
        },
        follow_ups: &[],
    },
    Case {
        name: "hold-the-draw-loses",
        start: Start::Fen("8/8/4k3/8/5K2/4P3/8/8 b - - 1 1", ""),
        ask: Ask::Mistake {
            played: Played::Worst,
            drill: Some("hold-the-draw"),
        },
        follow_ups: &[],
    },
    Case {
        name: "philidor-loses-the-draw",
        start: Start::Fen("3k4/8/3K4/3P4/8/8/6r1/5R2 b - - 0 1", ""),
        ask: Ask::Mistake {
            played: Played::Worst,
            drill: None,
        },
        follow_ups: &[],
    },
    Case {
        name: "opening-loses-a-pawn",
        start: Start::Moves("e4 e5 Nf3 Nc6 Bc4 Bc5"),
        ask: Ask::Mistake {
            played: Played::Uci("f3e5"),
            drill: None,
        },
        follow_ups: &[("What should I have played instead?", None)],
    },
];

/// Plies of a line worth keeping (`chess_core`'s coach takes ten).
const PLIES: usize = 10;
/// The depth a `Played::Worst` scan uses; it looks at every legal move.
const SCAN_DEPTH: u32 = 12;

fn main() {
    let depth: u32 = std::env::var("DEPTH")
        .ok()
        .and_then(|d| d.parse().ok())
        .unwrap_or(22);
    let mut engine = Engine::start();
    let mut out = Vec::new();
    for case in CASES {
        eprintln!("{}…", case.name);
        let (position, last_move) = start(&case.start);
        let fen = fen_of(&position);
        let body = match &case.ask {
            Ask::Explain => {
                let lines = engine.analyse(&fen, 3, depth);
                json!({
                    "fen": fen,
                    "last_move": last_move,
                    "lines": lines.iter().map(line_json).collect::<Vec<_>>(),
                })
            }
            Ask::Mistake { played, drill } => {
                let best = engine.analyse(&fen, 1, depth);
                let best = best.first().expect("a line from the engine");
                let played = match played {
                    Played::Uci(uci) => uci.parse::<UciMove>().expect("a UCI move"),
                    Played::Worst => engine.worst_move(&position),
                };
                let mv = played.to_move(&position).expect("the move played is legal");
                let mut after = position.clone();
                after.play_unchecked(mv);
                let (after_score, reply) = if after.is_game_over() {
                    (Value::Null, Vec::new())
                } else {
                    let reply = engine.analyse(&fen_of(&after), 1, depth);
                    let reply = reply.first().expect("a line from the engine");
                    (
                        score_json(reply.score.flip()),
                        reply.pv.iter().take(PLIES).map(uci_string).collect(),
                    )
                };
                json!({
                    "fen": fen,
                    "played": played.to_string(),
                    "better": best.pv.iter().take(PLIES).map(uci_string).collect::<Vec<_>>(),
                    "before": score_json(best.score),
                    "after": after_score,
                    "drill": drill,
                    "reply": reply,
                })
            }
        };
        let mut case_json = json!({ "name": case.name });
        let key = match case.ask {
            Ask::Explain => "explain",
            Ask::Mistake { .. } => "mistake",
        };
        case_json[key] = body;
        let follow_ups: Vec<Value> = case
            .follow_ups
            .iter()
            .map(|(question, about)| {
                let mut entry = json!({ "question": question });
                if let Some(san) = about {
                    let mv = san
                        .parse::<San>()
                        .expect("a move in SAN")
                        .to_move(&position)
                        .expect("a legal move to ask about");
                    let mut after = position.clone();
                    after.play_unchecked(mv);
                    let line = engine.analyse(&fen_of(&after), 1, depth);
                    let line = line.first().expect("a line from the engine");
                    entry["probe"] = json!({
                        "uci": uci_string(&mv.to_uci(CastlingMode::Standard)),
                        "line": line_json(line),
                    });
                }
                entry
            })
            .collect();
        if !follow_ups.is_empty() {
            case_json["follow_ups"] = Value::Array(follow_ups);
        }
        out.push(case_json);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&Value::Array(out)).expect("the cases serialise")
    );
}

/// The case's position, and the move that led to it in SAN.
fn start(start: &Start) -> (Chess, Option<String>) {
    let (mut position, moves): (Chess, Vec<String>) = match start {
        Start::Moves(moves) => (
            Chess::default(),
            moves.split_whitespace().map(str::to_string).collect(),
        ),
        Start::Fen(fen, moves) => (
            fen.parse::<Fen>()
                .expect("a FEN")
                .into_position(CastlingMode::Standard)
                .expect("a legal position"),
            moves.split_whitespace().map(str::to_string).collect(),
        ),
    };
    let mut last = None;
    for text in &moves {
        let mv = match start {
            Start::Moves(_) => text
                .parse::<San>()
                .expect("a move in SAN")
                .to_move(&position)
                .expect("a legal move"),
            Start::Fen(..) => text
                .parse::<UciMove>()
                .expect("a UCI move")
                .to_move(&position)
                .expect("a legal move"),
        };
        last = Some(SanPlus::from_move(position.clone(), mv).to_string());
        position.play_unchecked(mv);
    }
    (position, last)
}

fn fen_of(position: &Chess) -> String {
    Fen::from_position(position, EnPassantMode::Legal).to_string()
}

fn uci_string(uci: &UciMove) -> String {
    uci.to_string()
}

fn score_json(score: Score) -> Value {
    match score {
        Score::Cp(cp) => json!({ "kind": "cp", "value": cp }),
        Score::Mate(n) => json!({ "kind": "mate", "value": n }),
    }
}

fn line_json(line: &Line) -> Value {
    json!({
        "depth": line.depth,
        "score": score_json(line.score),
        "pv": line.pv.iter().take(PLIES).map(uci_string).collect::<Vec<_>>(),
    })
}

/// A local Stockfish, spoken to over UCI.
struct Engine {
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    _child: Child,
}

impl Engine {
    fn start() -> Engine {
        let path = std::env::var("STOCKFISH").unwrap_or_else(|_| "stockfish".to_string());
        let mut child = Command::new(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("{path}: {e} (set STOCKFISH to its path)"));
        let mut engine = Engine {
            stdin: child.stdin.take().expect("stdin"),
            out: BufReader::new(child.stdout.take().expect("stdout")),
            _child: child,
        };
        engine.send("uci");
        engine.read_until(|m| matches!(m, Message::UciOk));
        engine.send("setoption name Threads value 4");
        engine.send("setoption name Hash value 256");
        engine.send("isready");
        engine.read_until(|m| matches!(m, Message::ReadyOk));
        engine
    }

    fn send(&mut self, command: &str) {
        writeln!(self.stdin, "{command}").expect("the engine takes commands");
        self.stdin.flush().expect("the engine takes commands");
    }

    /// Read until `done` says so, collecting the complete lines seen.
    fn read_until(&mut self, done: impl Fn(&Message) -> bool) -> Vec<Line> {
        let mut lines = Vec::new();
        let mut text = String::new();
        loop {
            text.clear();
            let read = self.out.read_line(&mut text).expect("the engine answers");
            if read == 0 {
                panic!("the engine stopped");
            }
            let message = parse_message(&text);
            if let Message::Info(line) = &message {
                lines.push(line.clone());
            }
            if done(&message) {
                return lines;
            }
        }
    }

    /// The engine's best `multipv` lines for `fen`, deepest first seen last.
    fn analyse(&mut self, fen: &str, multipv: usize, depth: u32) -> Vec<Line> {
        self.send(&format!("setoption name MultiPV value {multipv}"));
        self.send(&format!("position fen {fen}"));
        self.send(&format!("go depth {depth}"));
        let seen = self.read_until(|m| matches!(m, Message::BestMove { .. }));
        // The last line for each rank is the deepest one.
        let mut best: Vec<Line> = Vec::new();
        for line in seen {
            match best.iter_mut().find(|l| l.multipv == line.multipv) {
                Some(slot) => *slot = line,
                None => best.push(line),
            }
        }
        best.sort_by_key(|l| l.multipv);
        best
    }

    /// The legal move that leaves the side to move worst off.
    fn worst_move(&mut self, position: &Chess) -> UciMove {
        let mut worst: Option<(i32, UciMove)> = None;
        for mv in position.legal_moves() {
            let mut after = position.clone();
            after.play_unchecked(mv);
            if after.is_game_over() {
                continue;
            }
            let lines = self.analyse(&fen_of(&after), 1, SCAN_DEPTH);
            let Some(line) = lines.first() else { continue };
            // The reply's score is from the opponent's side.
            let value = match line.score.flip() {
                Score::Cp(cp) => cp,
                Score::Mate(n) if n > 0 => 100_000 - n,
                Score::Mate(n) => -100_000 - n,
            };
            if worst.as_ref().is_none_or(|(low, _)| value < *low) {
                worst = Some((value, mv.to_uci(CastlingMode::Standard)));
            }
        }
        worst.expect("a legal move").1
    }
}
