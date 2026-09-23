//! JavaScript bindings for [`chess_core`], built with `wasm-pack` into
//! `web/src/lib/wasm`. The rule for this crate: data in, data out, and no
//! chess logic. If the UI needs a new fact, add it to `chess_core::Game`
//! with a test, then expose it here.
//!
//! Errors that mean "you can't do that" (illegal move, promotion needed)
//! come back as values, so the UI can react without try/catch. Errors that
//! mean "bad input" (unparseable FEN/PGN/square) throw a JS `Error`.

use chess_core::{
    GameError, GameStatus,
    engine::{self, Score},
    review,
    shakmaty::{Chess, Color, EnPassantMode, Position, Role, Square, fen::Fen, uci::UciMove},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[cfg(feature = "ts")]
use ts_rs::TS;

fn serializer() -> serde_wasm_bindgen::Serializer {
    // `null` for `None` and plain objects, matching the ts-rs bindings.
    serde_wasm_bindgen::Serializer::json_compatible()
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serializer())
        .map_err(|e| JsError::new(&e.to_string()))
}

fn parse_square(s: &str) -> Result<Square, JsError> {
    s.parse()
        .map_err(|_| JsError::new(&format!("not a square: {s:?}")))
}

fn parse_role(s: &str) -> Result<Role, JsError> {
    match s {
        "queen" => Ok(Role::Queen),
        "rook" => Ok(Role::Rook),
        "bishop" => Ok(Role::Bishop),
        "knight" => Ok(Role::Knight),
        _ => Err(JsError::new(&format!("not a promotion piece: {s:?}"))),
    }
}

fn game_error(e: GameError) -> JsError {
    JsError::new(&e.to_string())
}

// ----- view types -------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum Side {
    White,
    Black,
}

impl From<Color> for Side {
    fn from(c: Color) -> Self {
        if c.is_white() {
            Side::White
        } else {
            Side::Black
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum PieceRole {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl From<Role> for PieceRole {
    fn from(r: Role) -> Self {
        match r {
            Role::Pawn => PieceRole::Pawn,
            Role::Knight => PieceRole::Knight,
            Role::Bishop => PieceRole::Bishop,
            Role::Rook => PieceRole::Rook,
            Role::Queen => PieceRole::Queen,
            Role::King => PieceRole::King,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum Status {
    Ongoing,
    Check,
    Checkmate,
    Stalemate,
    InsufficientMaterial,
    FiftyMoveRule,
    ThreefoldRepetition,
}

impl From<GameStatus> for Status {
    fn from(s: GameStatus) -> Self {
        match s {
            GameStatus::Ongoing => Status::Ongoing,
            GameStatus::Check => Status::Check,
            GameStatus::Checkmate { .. } => Status::Checkmate,
            GameStatus::Stalemate => Status::Stalemate,
            GameStatus::InsufficientMaterial => Status::InsufficientMaterial,
            GameStatus::FiftyMoveRule => Status::FiftyMoveRule,
            GameStatus::ThreefoldRepetition => Status::ThreefoldRepetition,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct PieceOnSquare {
    pub square: String,
    pub color: Side,
    pub role: PieceRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct MoveSquares {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct MoveView {
    /// The ply this move leads to (1 for the first move).
    pub ply: u32,
    pub san: String,
    pub uci: String,
}

/// One element of the move tree written out like PGN (see
/// `chess_core::Game::tokens`), marked up for the move list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum TreeToken {
    Move {
        /// For `goToNode`, `deleteFrom`.
        id: u32,
        /// `3.`, or `3...` where a Black move needs one.
        number: Option<String>,
        san: String,
        /// Variation depth: 0 on the main line.
        depth: u32,
        /// The move at the cursor.
        current: bool,
        /// On the current line (the one Back/Forward walk).
        line: bool,
    },
    VariationStart,
    VariationEnd,
}

/// Everything the UI needs to draw the position at the cursor and the move
/// list. One of these per render; see `Game::view`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct GameView {
    pub fen: String,
    pub turn: Side,
    pub status: Status,
    /// Set only for a checkmate.
    pub winner: Option<Side>,
    pub game_over: bool,
    /// Half-moves from the start position to the viewed position.
    pub cursor: u32,
    pub ply_count: u32,
    pub viewing_history: bool,
    /// The move that led to the viewed position, as the user sees it
    /// (castling is a king move).
    pub last_move: Option<MoveSquares>,
    /// The square of the king in check, if any.
    pub check_square: Option<String>,
    pub pieces: Vec<PieceOnSquare>,
    /// The moves of the current line.
    pub moves: Vec<MoveView>,
    /// The whole tree, variations included, for the move list.
    pub tree: Vec<TreeToken>,
    /// The node at the cursor (`0` is the start position).
    pub node: u32,
    /// Whether the cursor is on the main line.
    pub main_line: bool,
    /// `1. e4 e5 (1... c5) 2. Nf3`, for the whole game regardless of the cursor.
    pub movetext: String,
    /// Minimal PGN export of the whole game (see `chess_core::Game::pgn`).
    pub pgn: String,
    pub start_turn: Side,
    pub start_fullmove: u32,
}

/// Outcome of trying to play a move from squares.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayResult {
    Ok = "ok",
    /// A promotion piece must be chosen and the move retried.
    PromotionRequired = "promotion_required",
    Illegal = "illegal",
}

fn play_result(r: Result<&chess_core::PlayedMove, GameError>) -> Result<PlayResult, JsError> {
    match r {
        Ok(_) => Ok(PlayResult::Ok),
        Err(GameError::IllegalMove) => Ok(PlayResult::Illegal),
        Err(GameError::PromotionRequired) => Ok(PlayResult::PromotionRequired),
        Err(e) => Err(game_error(e)),
    }
}

/// Everything `Game::view` reports, as a Rust value (also used by the tests).
fn game_view(g: &chess_core::Game) -> GameView {
    let pos = g.position();
    let status = g.status();
    let last_move = g.last_move().and_then(|m| match m.uci {
        UciMove::Normal { from, to, .. } => Some(MoveSquares {
            from: from.to_string(),
            to: to.to_string(),
        }),
        _ => None,
    });
    let check_square = pos
        .is_check()
        .then(|| pos.board().king_of(pos.turn()))
        .flatten()
        .map(|sq| sq.to_string());
    let pieces = pos
        .board()
        .iter()
        .map(|(square, piece)| PieceOnSquare {
            square: square.to_string(),
            color: piece.color.into(),
            role: piece.role.into(),
        })
        .collect();
    let moves = g
        .moves()
        .iter()
        .enumerate()
        .map(|(i, m)| MoveView {
            ply: i as u32 + 1,
            san: m.san.to_string(),
            uci: m.uci.to_string(),
        })
        .collect();
    let here = g.node();
    let tree = g
        .tokens()
        .into_iter()
        .map(|t| match t {
            chess_core::Token::Move {
                id,
                number,
                san,
                depth,
            } => TreeToken::Move {
                id: id as u32,
                number,
                san,
                depth,
                current: id == here,
                line: g.line().contains(&id),
            },
            chess_core::Token::VariationStart => TreeToken::VariationStart,
            chess_core::Token::VariationEnd => TreeToken::VariationEnd,
        })
        .collect();
    GameView {
        fen: g.fen(),
        turn: g.turn().into(),
        status: status.into(),
        winner: status.winner().map(Into::into),
        game_over: status.is_game_over(),
        cursor: g.cursor() as u32,
        ply_count: g.ply_count() as u32,
        viewing_history: g.is_viewing_history(),
        last_move,
        check_square,
        pieces,
        moves,
        tree,
        node: here as u32,
        main_line: g.is_main_line(here),
        movetext: g.movetext(),
        pgn: g.pgn(),
        start_turn: g.start_position().turn().into(),
        start_fullmove: g.start_position().fullmoves().get(),
    }
}

// ----- Game -------------------------------------------------------------

/// A chess game with history and a cursor. See `chess_core::Game`.
#[wasm_bindgen]
pub struct Game(chess_core::Game);

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Game {
        Game(chess_core::Game::new())
    }

    #[wasm_bindgen(js_name = fromFen)]
    pub fn from_fen(fen: &str) -> Result<Game, JsError> {
        chess_core::Game::from_fen(fen)
            .map(Game)
            .map_err(game_error)
    }

    /// The first game in a PGN, variations included.
    #[wasm_bindgen(js_name = fromPgn)]
    pub fn from_pgn(pgn: &str) -> Result<Game, JsError> {
        chess_core::Game::from_pgn(pgn)
            .map(Game)
            .map_err(game_error)
    }

    /// A `GameView` snapshot of the position at the cursor.
    pub fn view(&self) -> Result<JsValue, JsError> {
        to_js(&game_view(&self.0))
    }

    pub fn fen(&self) -> String {
        self.0.fen()
    }

    pub fn pgn(&self) -> String {
        self.0.pgn()
    }

    pub fn movetext(&self) -> String {
        self.0.movetext()
    }

    /// Squares the piece on `from` can legally move to in the viewed position.
    #[wasm_bindgen(js_name = legalDestinations)]
    pub fn legal_destinations(&self, from: &str) -> Result<Vec<String>, JsError> {
        let from = parse_square(from)?;
        Ok(self
            .0
            .legal_destinations(from)
            .into_iter()
            .map(|sq| sq.to_string())
            .collect())
    }

    /// Play at the end of the game. `promotion` is `"queen"`, `"rook"`,
    /// `"bishop"`, or `"knight"` when the move is a promotion.
    #[wasm_bindgen(js_name = playFromTo)]
    pub fn play_from_to(
        &mut self,
        from: &str,
        to: &str,
        promotion: Option<String>,
    ) -> Result<PlayResult, JsError> {
        let (from, to) = (parse_square(from)?, parse_square(to)?);
        let promotion = promotion.as_deref().map(parse_role).transpose()?;
        play_result(self.0.play_from_to(from, to, promotion))
    }

    /// Play from the viewed position, discarding any moves after it.
    #[wasm_bindgen(js_name = playHereFromTo)]
    pub fn play_here_from_to(
        &mut self,
        from: &str,
        to: &str,
        promotion: Option<String>,
    ) -> Result<PlayResult, JsError> {
        let (from, to) = (parse_square(from)?, parse_square(to)?);
        let promotion = promotion.as_deref().map(parse_role).transpose()?;
        play_result(self.0.play_here_from_to(from, to, promotion))
    }

    /// Play a UCI move (`e2e4`, `e7e8q`) at the end of the game.
    #[wasm_bindgen(js_name = playUci)]
    pub fn play_uci(&mut self, uci: &str) -> Result<PlayResult, JsError> {
        play_result(self.0.play_uci(uci))
    }

    /// Play a UCI move from the viewed position, discarding any moves after it.
    #[wasm_bindgen(js_name = playHereUci)]
    pub fn play_here_uci(&mut self, uci: &str) -> Result<PlayResult, JsError> {
        play_result(self.0.play_here_uci(uci))
    }

    /// View a move anywhere in the tree (its line becomes current).
    #[wasm_bindgen(js_name = goToNode)]
    pub fn go_to_node(&mut self, id: u32) {
        self.0.go_to_node(id as usize);
    }

    /// Switch to the next (`1`) or previous (`-1`) alternative to the move at
    /// the cursor.
    #[wasm_bindgen(js_name = switchVariation)]
    pub fn switch_variation(&mut self, step: i32) {
        self.0.switch_variation(step);
    }

    /// Promote the variation `id` is in one level. Whether anything changed.
    pub fn promote(&mut self, id: u32) -> bool {
        self.0.promote(id as usize)
    }

    /// Delete move `id` and everything after it. Whether anything changed.
    #[wasm_bindgen(js_name = deleteFrom)]
    pub fn delete_from(&mut self, id: u32) -> bool {
        self.0.delete_from(id as usize)
    }

    #[wasm_bindgen(js_name = goToPly)]
    pub fn go_to_ply(&mut self, ply: u32) {
        self.0.go_to_ply(ply as usize);
    }

    #[wasm_bindgen(js_name = goBack)]
    pub fn go_back(&mut self) {
        self.0.go_back();
    }

    #[wasm_bindgen(js_name = goForward)]
    pub fn go_forward(&mut self) {
        self.0.go_forward();
    }

    #[wasm_bindgen(js_name = goToStart)]
    pub fn go_to_start(&mut self) {
        self.0.go_to_start();
    }

    #[wasm_bindgen(js_name = goToEnd)]
    pub fn go_to_end(&mut self) {
        self.0.go_to_end();
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

// ----- engine output ----------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum EngineScore {
    Cp(i32),
    Mate(i32),
}

impl From<Score> for EngineScore {
    fn from(s: Score) -> Self {
        match s {
            Score::Cp(cp) => EngineScore::Cp(cp),
            Score::Mate(n) => EngineScore::Mate(n),
        }
    }
}

impl From<EngineScore> for Score {
    fn from(s: EngineScore) -> Self {
        match s {
            EngineScore::Cp(cp) => Score::Cp(cp),
            EngineScore::Mate(n) => Score::Mate(n),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct EngineLine {
    pub depth: u32,
    pub multipv: u32,
    /// From the side to move's point of view (see `scoreForWhite`).
    pub score: EngineScore,
    pub pv: Vec<String>,
}

/// One line of UCI output, classified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum EngineMessage {
    UciOk,
    ReadyOk,
    Info { line: EngineLine },
    BestMove { best: Option<String> },
    Other { text: String },
}

/// Classify one line of UCI engine output (`EngineMessage`).
#[wasm_bindgen(js_name = parseEngineMessage)]
pub fn parse_engine_message(line: &str) -> Result<JsValue, JsError> {
    let msg = match engine::parse_message(line) {
        engine::Message::UciOk => EngineMessage::UciOk,
        engine::Message::ReadyOk => EngineMessage::ReadyOk,
        engine::Message::Info(l) => EngineMessage::Info {
            line: EngineLine {
                depth: l.depth,
                multipv: l.multipv,
                score: l.score.into(),
                pv: l.pv.iter().map(ToString::to_string).collect(),
            },
        },
        engine::Message::BestMove { best, .. } => EngineMessage::BestMove {
            best: best.map(|m| m.to_string()),
        },
        engine::Message::Other(text) => EngineMessage::Other { text },
    };
    to_js(&msg)
}

fn score_from_js(score: JsValue) -> Result<Score, JsError> {
    let score: EngineScoreIn =
        serde_wasm_bindgen::from_value(score).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(match score.kind.as_str() {
        "cp" => Score::Cp(score.value),
        "mate" => Score::Mate(score.value),
        other => return Err(JsError::new(&format!("unknown score kind {other:?}"))),
    })
}

#[derive(serde::Deserialize)]
struct EngineScoreIn {
    kind: String,
    value: i32,
}

fn parse_side(side: &str) -> Result<Color, JsError> {
    match side {
        "white" => Ok(Color::White),
        "black" => Ok(Color::Black),
        other => Err(JsError::new(&format!("not a side: {other:?}"))),
    }
}

/// Convert an `EngineScore` given for the side to move (`turn`) into White's
/// point of view.
#[wasm_bindgen(js_name = scoreForWhite)]
pub fn score_for_white(score: JsValue, turn: &str) -> Result<JsValue, JsError> {
    let turn = parse_side(turn)?;
    to_js(&EngineScore::from(score_from_js(score)?.for_white(turn)))
}

/// Share of an eval bar (0 to 1) to fill for the side an `EngineScore` favours.
#[wasm_bindgen(js_name = barFraction)]
pub fn bar_fraction(score: JsValue) -> Result<f64, JsError> {
    Ok(score_from_js(score)?.bar_fraction())
}

/// Whether a move that took the evaluation from `before` to `after` (both
/// `EngineScore`s from the mover's point of view) was a mistake.
#[wasm_bindgen(js_name = isMistake)]
pub fn is_mistake(before: JsValue, after: JsValue) -> Result<bool, JsError> {
    Ok(Score::is_mistake(
        score_from_js(before)?,
        score_from_js(after)?,
    ))
}

/// `+0.35`, `-1.20`, `#3`.
#[wasm_bindgen(js_name = formatScore)]
pub fn format_score(score: JsValue) -> Result<String, JsError> {
    Ok(score_from_js(score)?.to_string())
}

/// Judge a puzzle move (`Verdict`): `moves` is the puzzle as published (the
/// setup move first), `played` every move made since the setup, ending with
/// the solver's newest.
#[wasm_bindgen(js_name = judgePuzzle)]
pub fn judge_puzzle(
    fen: &str,
    moves: Vec<String>,
    played: Vec<String>,
) -> Result<JsValue, JsError> {
    let moves: Vec<&str> = moves.iter().map(String::as_str).collect();
    let played: Vec<&str> = played.iter().map(String::as_str).collect();
    let puzzle =
        chess_core::puzzle::Puzzle::new(fen, &moves).map_err(|e| JsError::new(&e.to_string()))?;
    let verdict = puzzle
        .judge(&played)
        .map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&verdict)
}

/// Every lesson drill (`Drill[]`), easiest first.
#[wasm_bindgen]
pub fn drills() -> Result<JsValue, JsError> {
    to_js(&chess_core::lesson::drills())
}

/// Where drill `id` stands after `moves` (UCI, both sides): `DrillStatus`.
#[wasm_bindgen(js_name = assessDrill)]
pub fn assess_drill(id: &str, moves: Vec<String>) -> Result<JsValue, JsError> {
    let drill =
        chess_core::lesson::find(id).ok_or_else(|| JsError::new(&format!("no drill {id:?}")))?;
    let moves: Vec<&str> = moves.iter().map(String::as_str).collect();
    let status =
        chess_core::lesson::assess(&drill, &moves).map_err(|e| JsError::new(&e.to_string()))?;
    to_js(&status)
}

// ----- game review ------------------------------------------------------

/// What the engine made of one position of a game under review: its score
/// for the side to move there and its line (UCI), best move first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct PositionEval {
    pub score: EngineScore,
    pub pv: Vec<String>,
}

/// One of a game's biggest swings (`chess_core::review::Swing`), for display.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct ReviewSwing {
    /// The ply the move leads to (1 for the first move).
    pub ply: u32,
    pub mover: Side,
    /// The position the move was played from.
    pub fen: String,
    /// The move played, numbered: `12... Qxd4`.
    pub played: String,
    pub played_uci: String,
    /// The evaluation before and after the move, from White's point of view.
    pub before: EngineScore,
    pub after: EngineScore,
    /// The mover's share of an eval bar (0 to 1) before and after the move.
    pub before_chance: f64,
    pub after_chance: f64,
    /// The engine's better line, numbered: `12... Nf6 13. Bd3`.
    pub best: String,
    pub best_uci: Vec<String>,
}

/// A finished game's review: its biggest swings, biggest first, and the game
/// as PGN with each swing's better line added as a variation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Review {
    pub swings: Vec<ReviewSwing>,
    pub pgn: String,
}

fn review(
    game: &chess_core::Game,
    evals: &[Option<PositionEval>],
    side: Option<Color>,
) -> Result<Review, String> {
    let evals = evals
        .iter()
        .map(|e| {
            e.as_ref()
                .map(|e| {
                    let pv =
                        e.pv.iter()
                            .map(|m| m.parse().map_err(|_| format!("not a UCI move: {m:?}")))
                            .collect::<Result<_, _>>()?;
                    Ok(review::Evaluation {
                        score: e.score.into(),
                        pv,
                    })
                })
                .transpose()
        })
        .collect::<Result<Vec<_>, String>>()?;
    let swings = review::swings(game, &evals, side, review::SWINGS);
    let pgn = review::with_lines(game, &swings).pgn();
    let swings = swings
        .iter()
        .map(|s| ReviewSwing {
            ply: s.ply as u32,
            mover: s.mover.into(),
            fen: Fen::from_position(&s.before_position, EnPassantMode::Legal).to_string(),
            played: s.played_movetext(),
            played_uci: s.played_uci.to_string(),
            before: s.before.for_white(s.mover).into(),
            after: s.after.for_white(s.mover).into(),
            before_chance: s.before.bar_fraction(),
            after_chance: s.after.bar_fraction(),
            best: s.best_movetext(),
            best_uci: s.best.iter().map(ToString::to_string).collect(),
        })
        .collect();
    Ok(Review { swings, pgn })
}

/// Review a finished game (`Review`): `pgn` is the game, `evals` a
/// `PositionEval | null` for each position of its main line from the start
/// (fewer if the analysis was cut short), and `side` keeps only that
/// player's moves.
#[wasm_bindgen(js_name = reviewGame)]
pub fn review_game(pgn: &str, evals: JsValue, side: Option<String>) -> Result<JsValue, JsError> {
    let game = chess_core::Game::from_pgn(pgn).map_err(game_error)?;
    let evals: Vec<Option<PositionEval>> =
        serde_wasm_bindgen::from_value(evals).map_err(|e| JsError::new(&e.to_string()))?;
    let side = side.as_deref().map(parse_side).transpose()?;
    to_js(&review(&game, &evals, side).map_err(|e| JsError::new(&e))?)
}

/// A principal variation as numbered SAN movetext from the position `fen`.
#[wasm_bindgen(js_name = pvMovetext)]
pub fn pv_movetext(fen: &str, pv: Vec<String>) -> Result<String, JsError> {
    let pos: Chess = fen
        .parse::<Fen>()
        .map_err(|e| JsError::new(&e.to_string()))?
        .into_position(chess_core::shakmaty::CastlingMode::Standard)
        .map_err(|e| JsError::new(&e.to_string()))?;
    let pv: Vec<UciMove> = pv
        .iter()
        .map(|m| {
            m.parse()
                .map_err(|_| JsError::new(&format!("not a UCI move: {m:?}")))
        })
        .collect::<Result<_, _>>()?;
    Ok(engine::pv_movetext(&pos, &pv))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The JS-facing behaviour is covered by web/src/lib/chess/game.svelte.spec.ts;
    // these check the pure parts natively.
    #[test]
    fn view_of_a_short_game_serializes_to_the_documented_shape() {
        let mut g = chess_core::Game::new();
        g.play_uci("e2e4").unwrap();
        g.play_uci("e7e5").unwrap();
        let game = Game(g);
        let json = serde_json::to_value(GameViewProbe::from(&game)).unwrap();
        assert_eq!(json["turn"], "white");
        assert_eq!(json["status"], "ongoing");
        assert_eq!(json["cursor"], 2);
        assert_eq!(json["lastMove"]["from"], "e7");
        assert_eq!(json["moves"][1]["san"], "e5");
        assert_eq!(json["movetext"], "1. e4 e5");
        assert_eq!(json["pgn"], "1. e4 e5 *");
        assert_eq!(json["pieces"].as_array().unwrap().len(), 32);
        assert_eq!(json["winner"], serde_json::Value::Null);
    }

    #[test]
    fn engine_message_shape() {
        let msg = match engine::parse_message("info depth 12 multipv 1 score cp 35 pv e2e4 e7e5") {
            engine::Message::Info(l) => EngineMessage::Info {
                line: EngineLine {
                    depth: l.depth,
                    multipv: l.multipv,
                    score: l.score.into(),
                    pv: l.pv.iter().map(ToString::to_string).collect(),
                },
            },
            _ => panic!(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(
            json,
            r#"{"type":"info","line":{"depth":12,"multipv":1,"score":{"kind":"cp","value":35},"pv":["e2e4","e7e5"]}}"#
        );
    }

    #[test]
    fn review_shape() {
        let game = chess_core::Game::from_pgn("1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7#").unwrap();
        let cp = |value: i32, pv: &[&str]| {
            Some(PositionEval {
                score: EngineScore::Cp(value),
                pv: pv.iter().map(ToString::to_string).collect(),
            })
        };
        let evals = vec![
            cp(30, &["e2e4"]),
            cp(-30, &["e7e5"]),
            cp(30, &["g1f3"]),
            cp(-10, &["b8c6"]),
            cp(40, &["f1c4"]),
            cp(-60, &["g7g6", "h5f3"]),
            Some(PositionEval {
                score: EngineScore::Mate(1),
                pv: vec!["h5f7".into()],
            }),
            None,
        ];
        let r = review(&game, &evals, None).unwrap();
        assert_eq!(
            r.pgn,
            "1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 (3... g6 4. Qf3) 4. Qxf7# 1-0"
        );
        let json = serde_json::to_value(&r.swings[0]).unwrap();
        assert_eq!(json["ply"], 6);
        assert_eq!(json["mover"], "black");
        assert_eq!(json["played"], "3... Nf6");
        assert_eq!(json["playedUci"], "g8f6");
        assert_eq!(json["best"], "3... g6 4. Qf3");
        // White's point of view: Black was a little better, then White mates.
        assert_eq!(
            json["before"],
            serde_json::json!({"kind": "cp", "value": 60})
        );
        assert_eq!(
            json["after"],
            serde_json::json!({"kind": "mate", "value": 1})
        );
        assert_eq!(json["afterChance"], 0.0);
        assert!(
            review(&game, &evals, Some(Color::White))
                .unwrap()
                .swings
                .is_empty()
        );
        let garbage = vec![cp(0, &["xyz"])];
        assert!(review(&game, &garbage, None).is_err());
    }

    /// Builds the same `GameView` as `Game::view` without going through JS.
    struct GameViewProbe;
    impl GameViewProbe {
        fn from(game: &Game) -> GameView {
            game_view(&game.0)
        }
    }
}
