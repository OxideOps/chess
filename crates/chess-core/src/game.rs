use std::fmt::Write as _;

use shakmaty::{
    CastlingMode, Chess, Color, EnPassantMode, Move, Position, Role, Square, fen::Fen,
    san::SanPlus, uci::UciMove, zobrist::Zobrist64,
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GameError {
    #[error("illegal move")]
    IllegalMove,
    #[error("a promotion piece must be chosen for this move")]
    PromotionRequired,
    #[error("invalid FEN: {0}")]
    InvalidFen(String),
    #[error("invalid UCI move: {0}")]
    InvalidUci(String),
    #[error("invalid PGN: {0}")]
    InvalidPgn(String),
}

/// The state of the position currently being viewed.
///
/// Fifty-move and threefold repetition are reported as terminal here. Under
/// FIDE rules they are only *claimable* draws (75 moves / fivefold are
/// automatic); whether to auto-apply them is a product decision for the
/// server, which can check `halfmoves()` and `repetition_count()` itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Ongoing,
    Check,
    Checkmate { winner: Color },
    Stalemate,
    InsufficientMaterial,
    FiftyMoveRule,
    ThreefoldRepetition,
}

impl GameStatus {
    pub fn is_game_over(self) -> bool {
        !matches!(self, GameStatus::Ongoing | GameStatus::Check)
    }

    /// `Some(winner)` for a decisive result, `None` for a draw or an unfinished game.
    pub fn winner(self) -> Option<Color> {
        match self {
            GameStatus::Checkmate { winner } => Some(winner),
            _ => None,
        }
    }
}

/// A move together with the notations the UI and the wire need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayedMove {
    pub mv: Move,
    pub san: SanPlus,
    pub uci: UciMove,
}

/// A chess game: a start position, the moves played from it, and a cursor for
/// stepping back through the history.
///
/// Moves are always appended to the *end* of the game regardless of where the
/// cursor is (this is a game record, not an analysis tree). Playing a move
/// snaps the cursor back to the end.
#[derive(Debug, Clone)]
pub struct Game {
    start_fen: Fen,
    /// `positions[i]` is the position after `i` moves; `positions[0]` is the start.
    positions: Vec<Chess>,
    moves: Vec<PlayedMove>,
    /// Index into `positions` of the position being viewed.
    cursor: usize,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self::from_position(Chess::default())
    }

    pub fn from_fen(fen: &str) -> Result<Self, GameError> {
        let fen: Fen = fen
            .parse()
            .map_err(|e| GameError::InvalidFen(format!("{e}")))?;
        let pos: Chess = fen
            .into_position(CastlingMode::Standard)
            .map_err(|e| GameError::InvalidFen(format!("{e}")))?;
        Ok(Self::from_position(pos))
    }

    /// Load the main line of the first game in a PGN. See [`crate::pgn`].
    pub fn from_pgn(pgn: &str) -> Result<Self, GameError> {
        crate::pgn::parse(pgn).map(|p| p.game)
    }

    fn from_position(pos: Chess) -> Self {
        Self {
            start_fen: Fen::from_position(&pos, EnPassantMode::Legal),
            positions: vec![pos],
            moves: Vec::new(),
            cursor: 0,
        }
    }

    // ----- reading state -------------------------------------------------

    /// The position at the cursor.
    pub fn position(&self) -> &Chess {
        &self.positions[self.cursor]
    }

    /// The position the game started from.
    pub fn start_position(&self) -> &Chess {
        &self.positions[0]
    }

    /// The position after the last move played, ignoring the cursor.
    pub fn latest(&self) -> &Chess {
        self.positions.last().expect("positions is never empty")
    }

    pub fn start_fen(&self) -> String {
        self.start_fen.to_string()
    }

    /// FEN of the position at the cursor.
    pub fn fen(&self) -> String {
        Fen::from_position(self.position(), EnPassantMode::Legal).to_string()
    }

    pub fn moves(&self) -> &[PlayedMove] {
        &self.moves
    }

    /// Number of half-moves played (not counting anything before the start position).
    pub fn ply_count(&self) -> usize {
        self.moves.len()
    }

    /// Number of half-moves from the start to the cursor.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_viewing_history(&self) -> bool {
        self.cursor != self.moves.len()
    }

    /// The move that led to the position at the cursor, if any.
    pub fn last_move(&self) -> Option<&PlayedMove> {
        self.cursor.checked_sub(1).map(|i| &self.moves[i])
    }

    pub fn turn(&self) -> Color {
        self.position().turn()
    }

    /// Status of the position at the cursor.
    pub fn status(&self) -> GameStatus {
        self.status_at(self.cursor)
    }

    /// Status of the position after the last move, ignoring the cursor.
    pub fn final_status(&self) -> GameStatus {
        self.status_at(self.moves.len())
    }

    fn status_at(&self, ply: usize) -> GameStatus {
        let pos = &self.positions[ply];
        if pos.is_checkmate() {
            GameStatus::Checkmate {
                winner: !pos.turn(),
            }
        } else if pos.is_stalemate() {
            GameStatus::Stalemate
        } else if pos.is_insufficient_material() {
            GameStatus::InsufficientMaterial
        } else if pos.halfmoves() >= 100 {
            GameStatus::FiftyMoveRule
        } else if self.repetition_count_at(ply) >= 3 {
            GameStatus::ThreefoldRepetition
        } else if pos.is_check() {
            GameStatus::Check
        } else {
            GameStatus::Ongoing
        }
    }

    /// How many times the position at the cursor has occurred so far
    /// (including this occurrence).
    pub fn repetition_count(&self) -> usize {
        self.repetition_count_at(self.cursor)
    }

    fn repetition_count_at(&self, ply: usize) -> usize {
        let hash = self.positions[ply].zobrist_hash::<Zobrist64>(EnPassantMode::Legal);
        self.positions[..=ply]
            .iter()
            .filter(|p| p.zobrist_hash::<Zobrist64>(EnPassantMode::Legal) == hash)
            .count()
    }

    /// Squares the piece on `from` can legally move to in the position at the
    /// cursor. Castling is reported as the king's destination square (g1/c1),
    /// matching what a user would drag to.
    pub fn legal_destinations(&self, from: Square) -> Vec<Square> {
        self.position()
            .legal_moves()
            .iter()
            .filter_map(uci_squares)
            .filter(|(f, _)| *f == from)
            .map(|(_, to)| to)
            .collect()
    }

    // ----- playing moves -------------------------------------------------

    /// Play a fully specified move at the end of the game.
    pub fn play(&mut self, mv: Move) -> Result<&PlayedMove, GameError> {
        let pos = self.latest().clone();
        if !pos.is_legal(mv) {
            return Err(GameError::IllegalMove);
        }
        let uci = UciMove::from_move(mv, CastlingMode::Standard);
        let san = SanPlus::from_move(pos.clone(), mv);
        let next = pos.play(mv).map_err(|_| GameError::IllegalMove)?;

        self.positions.push(next);
        self.moves.push(PlayedMove { mv, san, uci });
        self.cursor = self.moves.len();
        Ok(self.moves.last().expect("just pushed"))
    }

    /// Play a move given as the user sees it: a source square, a destination
    /// square, and (only when the move is a pawn promotion) the piece to
    /// promote to. Castling is `e1 -> g1` style.
    ///
    /// Returns [`GameError::PromotionRequired`] when the move is a promotion
    /// and `promotion` is `None`, so the UI can prompt for a piece.
    pub fn play_from_to(
        &mut self,
        from: Square,
        to: Square,
        promotion: Option<Role>,
    ) -> Result<&PlayedMove, GameError> {
        let mv = resolve_move(self.latest(), from, to, promotion)?;
        self.play(mv)
    }

    /// Like [`Game::play_from_to`], but from the position at the cursor: any
    /// moves after the cursor are discarded first. Nothing changes if the
    /// move is illegal or still needs a promotion piece.
    pub fn play_here_from_to(
        &mut self,
        from: Square,
        to: Square,
        promotion: Option<Role>,
    ) -> Result<&PlayedMove, GameError> {
        let mv = resolve_move(self.position(), from, to, promotion)?;
        self.truncate_to_cursor();
        self.play(mv)
    }

    /// Play a UCI move from the position at the cursor, discarding any moves
    /// after it. Used to follow an engine line.
    pub fn play_here_uci(&mut self, uci: &str) -> Result<&PlayedMove, GameError> {
        let uci: UciMove = uci
            .parse()
            .map_err(|_| GameError::InvalidUci(uci.to_string()))?;
        let mv = uci
            .to_move(self.position())
            .map_err(|_| GameError::IllegalMove)?;
        self.truncate_to_cursor();
        self.play(mv)
    }

    /// Play a move in UCI notation (`e2e4`, `e7e8q`), e.g. from the wire or an engine.
    pub fn play_uci(&mut self, uci: &str) -> Result<&PlayedMove, GameError> {
        let uci: UciMove = uci
            .parse()
            .map_err(|_| GameError::InvalidUci(uci.to_string()))?;
        let mv = uci
            .to_move(self.latest())
            .map_err(|_| GameError::IllegalMove)?;
        self.play(mv)
    }

    /// Forget every move after the cursor, so the viewed position becomes the
    /// end of the game. This is how an analysis board "plays from here".
    pub fn truncate_to_cursor(&mut self) {
        self.positions.truncate(self.cursor + 1);
        self.moves.truncate(self.cursor);
    }

    // ----- navigation ----------------------------------------------------

    pub fn go_to_ply(&mut self, ply: usize) {
        self.cursor = ply.min(self.moves.len());
    }

    pub fn go_back(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn go_forward(&mut self) {
        self.go_to_ply(self.cursor + 1);
    }

    pub fn go_to_start(&mut self) {
        self.cursor = 0;
    }

    pub fn go_to_end(&mut self) {
        self.cursor = self.moves.len();
    }

    // ----- export --------------------------------------------------------

    /// PGN movetext without headers or result, e.g. `1. e4 e5 2. Nf3`.
    /// Starts from the game's start position, so a game that began from a
    /// FEN with Black to move opens with `1... e5`.
    pub fn movetext(&self) -> String {
        write_movetext(&self.positions[0], self.moves.iter().map(|m| &m.san))
    }

    /// The result token for the game as played: `1-0`, `0-1`, `1/2-1/2`, or
    /// `*` while it is unfinished.
    pub fn result_token(&self) -> &'static str {
        let status = self.final_status();
        match status.winner() {
            Some(Color::White) => "1-0",
            Some(Color::Black) => "0-1",
            None if status.is_game_over() => "1/2-1/2",
            None => "*",
        }
    }

    /// A minimal PGN export: `SetUp`/`FEN` tags when the game didn't start
    /// from the initial position, the movetext, and the result token.
    pub fn pgn(&self) -> String {
        let mut out = String::new();
        if *self.start_position() != Chess::default() {
            let _ = writeln!(out, "[SetUp \"1\"]");
            let _ = writeln!(out, "[FEN \"{}\"]", self.start_fen());
            out.push('\n');
        }
        let movetext = self.movetext();
        if !movetext.is_empty() {
            out.push_str(&movetext);
            out.push(' ');
        }
        out.push_str(self.result_token());
        out
    }
}

/// Number a sequence of SAN moves starting from `start`, e.g. `1. e4 e5 2. Nf3`
/// or `1... e5 2. Nf3` when Black moves first.
pub(crate) fn write_movetext<'a>(start: &Chess, sans: impl Iterator<Item = &'a SanPlus>) -> String {
    let mut out = String::new();
    let mut number = start.fullmoves().get();
    for (i, san) in sans.enumerate() {
        let white_to_move = (i % 2 == 0) == (start.turn() == Color::White);
        if white_to_move {
            let _ = write!(out, "{number}. ");
        } else if i == 0 {
            let _ = write!(out, "{number}... ");
        }
        let _ = write!(out, "{san} ");
        if !white_to_move {
            number += 1;
        }
    }
    out.trim_end().to_string()
}

/// Find the legal move in `pos` matching a from/to pair and optional promotion.
fn resolve_move(
    pos: &Chess,
    from: Square,
    to: Square,
    promotion: Option<Role>,
) -> Result<Move, GameError> {
    let candidates: Vec<Move> = pos
        .legal_moves()
        .iter()
        .copied()
        .filter(|m| uci_squares(m) == Some((from, to)))
        .collect();

    match candidates.as_slice() {
        [] => Err(GameError::IllegalMove),
        [single] if !single.is_promotion() => Ok(*single),
        _ => {
            let role = promotion.ok_or(GameError::PromotionRequired)?;
            candidates
                .iter()
                .find(|m| m.promotion() == Some(role))
                .copied()
                .ok_or(GameError::IllegalMove)
        }
    }
}

/// The (from, to) squares of a move as UCI/users see them, with castling as a
/// king move (`e1 -> g1`). `None` for drops, which don't occur in standard chess.
fn uci_squares(m: &Move) -> Option<(Square, Square)> {
    match UciMove::from_move(*m, CastlingMode::Standard) {
        UciMove::Normal { from, to, .. } => Some((from, to)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    fn game(moves: &str) -> Game {
        let mut g = Game::new();
        for m in moves.split_whitespace() {
            g.play_uci(m).unwrap_or_else(|e| panic!("{m}: {e}"));
        }
        g
    }

    fn sq(s: &str) -> Square {
        s.parse().unwrap()
    }

    fn sorted(mut squares: Vec<Square>) -> Vec<Square> {
        squares.sort();
        squares
    }

    #[test]
    fn library_is_wired_up_correctly() {
        assert_eq!(shakmaty::perft(&Chess::default(), 4), 197_281);
    }

    #[test]
    fn start_position_fen() {
        assert_eq!(Game::new().fen(), START_FEN);
        assert_eq!(Game::from_fen(START_FEN).unwrap().fen(), START_FEN);
        assert!(matches!(
            Game::from_fen("not a fen"),
            Err(GameError::InvalidFen(_))
        ));
    }

    #[test]
    fn san_uses_algebraic_notation() {
        let g = game("e2e4 d7d5 e4d5 d8d5 b1c3 d5e5 f1e2 e5e2");
        assert_eq!(
            g.movetext(),
            "1. e4 d5 2. exd5 Qxd5 3. Nc3 Qe5+ 4. Be2 Qxe2+"
        );
        assert_eq!(g.status(), GameStatus::Check);

        let g = game("e2e4 e7e5 f1c4 b8c6 d1h5 g8f6 h5f7");
        assert_eq!(g.movetext(), "1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6 4. Qxf7#");
        assert_eq!(
            g.status(),
            GameStatus::Checkmate {
                winner: Color::White
            }
        );
        assert!(g.status().is_game_over());
        assert_eq!(g.status().winner(), Some(Color::White));
    }

    // Regression for a v1 bug: any piece landing on the en passant square
    // removed the pawn behind it.
    #[test]
    fn only_pawns_capture_en_passant() {
        let g = game("e2e4 a7a5 f1a6");
        assert_eq!(
            g.position().board().piece_at(sq("a5")),
            Some(Color::Black.pawn())
        );

        let g = game("e2e4 a7a6 e4e5 d7d5 e5d6");
        assert_eq!(g.position().board().piece_at(sq("d5")), None);
        assert_eq!(g.moves().last().unwrap().san.to_string(), "exd6");
    }

    // Regression for a v1 bug: castling was allowed while in check.
    #[test]
    fn cannot_castle_out_of_check() {
        // 1. e4 e5 2. Nf3 Bc5 3. Bc4 Bxf2+ : f1/g1 are empty but the king is in check.
        let g = game("e2e4 e7e5 g1f3 f8c5 f1c4 c5f2");
        assert_eq!(g.status(), GameStatus::Check);
        assert_eq!(
            sorted(g.legal_destinations(sq("e1"))),
            sorted(vec![sq("e2"), sq("f1"), sq("f2")])
        );
    }

    #[test]
    fn castling_is_a_king_move_in_the_ui() {
        let mut g = game("e2e4 e7e5 g1f3 b8c6 f1c4 f8c5");
        assert!(g.legal_destinations(sq("e1")).contains(&sq("g1")));
        g.play_from_to(sq("e1"), sq("g1"), None).unwrap();
        assert_eq!(g.moves().last().unwrap().san.to_string(), "O-O");
        assert_eq!(g.moves().last().unwrap().uci.to_string(), "e1g1");
    }

    #[test]
    fn promotion_must_be_chosen_and_can_underpromote() {
        let mut g = game("a2a4 b7b5 a4b5 a7a6 b5a6 b8c6 a6a7 c6b8");
        assert_eq!(
            g.play_from_to(sq("a7"), sq("b8"), None),
            Err(GameError::PromotionRequired)
        );
        g.play_from_to(sq("a7"), sq("b8"), Some(Role::Knight))
            .unwrap();
        assert_eq!(g.moves().last().unwrap().san.to_string(), "axb8=N");
        assert_eq!(
            g.position().board().piece_at(sq("b8")),
            Some(Color::White.knight())
        );
    }

    #[test]
    fn illegal_moves_are_rejected() {
        let mut g = Game::new();
        assert_eq!(g.play_uci("e2e5"), Err(GameError::IllegalMove));
        assert_eq!(g.play_uci("e7e5"), Err(GameError::IllegalMove));
        assert!(matches!(g.play_uci("zz"), Err(GameError::InvalidUci(_))));
        assert_eq!(
            g.play_from_to(sq("e2"), sq("e5"), None),
            Err(GameError::IllegalMove)
        );
        assert_eq!(g.ply_count(), 0);
    }

    #[test]
    fn threefold_repetition() {
        let g = game("g1f3 g8f6 f3g1 f6g8 g1f3 g8f6 f3g1 f6g8");
        assert_eq!(g.repetition_count(), 3);
        assert_eq!(g.status(), GameStatus::ThreefoldRepetition);
        assert!(g.status().is_game_over());
    }

    #[test]
    fn navigation_views_history_without_changing_it() {
        let mut g = game("e2e4 e7e5 g1f3");
        g.go_back();
        assert!(g.is_viewing_history());
        assert_eq!(g.cursor(), 2);
        assert_eq!(g.turn(), Color::White);
        assert_eq!(g.last_move().unwrap().uci.to_string(), "e7e5");
        assert_eq!(
            sorted(g.legal_destinations(sq("g1"))),
            sorted(vec![sq("e2"), sq("f3"), sq("h3")])
        );

        g.go_to_start();
        assert_eq!(g.fen(), START_FEN);
        g.go_forward();
        assert_eq!(g.cursor(), 1);
        g.go_to_end();
        assert!(!g.is_viewing_history());
        assert_eq!(g.ply_count(), 3);

        // Playing while viewing history appends to the real end of the game.
        g.go_to_start();
        g.play_uci("b8c6").unwrap();
        assert_eq!(g.movetext(), "1. e4 e5 2. Nf3 Nc6");
        assert!(!g.is_viewing_history());
    }

    #[test]
    fn truncate_to_cursor_plays_from_here() {
        let mut g = game("e2e4 e7e5 g1f3 b8c6");
        g.go_to_ply(2);
        g.truncate_to_cursor();
        assert_eq!(g.ply_count(), 2);
        assert!(!g.is_viewing_history());
        assert_eq!(g.movetext(), "1. e4 e5");
        g.play_uci("f1c4").unwrap();
        assert_eq!(g.movetext(), "1. e4 e5 2. Bc4");

        // At the end it is a no-op; at the start it clears the game.
        g.truncate_to_cursor();
        assert_eq!(g.ply_count(), 3);
        g.go_to_start();
        g.truncate_to_cursor();
        assert_eq!(g.ply_count(), 0);
        assert_eq!(g.fen(), START_FEN);
    }

    #[test]
    fn playing_from_history_discards_the_future() {
        let mut g = game("e2e4 e7e5 g1f3 b8c6");
        g.go_to_ply(1);
        // An illegal move changes nothing, not even the history.
        assert_eq!(
            g.play_here_from_to(sq("e7"), sq("e4"), None),
            Err(GameError::IllegalMove)
        );
        assert_eq!(g.ply_count(), 4);
        assert_eq!(g.cursor(), 1);

        g.play_here_from_to(sq("c7"), sq("c5"), None).unwrap();
        assert_eq!(g.movetext(), "1. e4 c5");
        assert!(!g.is_viewing_history());

        g.go_to_start();
        g.play_here_uci("d2d4").unwrap();
        assert_eq!(g.movetext(), "1. d4");
        assert_eq!(g.play_here_uci("d2d4"), Err(GameError::IllegalMove));
    }

    #[test]
    fn pgn_export_and_result() {
        let g = game("e2e4 e7e5 f1c4 b8c6 d1h5 g8f6 h5f7");
        assert_eq!(g.result_token(), "1-0");
        assert_eq!(g.pgn(), "1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6 4. Qxf7# 1-0");
        assert_eq!(Game::new().pgn(), "*");

        let mut g = game("e2e4 e7e5");
        g.go_to_start();
        // The result describes the whole game, not the viewed position.
        assert_eq!(g.result_token(), "*");
        assert_eq!(g.final_status(), GameStatus::Ongoing);

        let mut g =
            Game::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap();
        g.play_uci("e7e5").unwrap();
        assert_eq!(
            g.pgn(),
            "[SetUp \"1\"]\n[FEN \"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1\"]\n\n1... e5 *"
        );
        // Round trip.
        assert_eq!(Game::from_pgn(&g.pgn()).unwrap().pgn(), g.pgn());
    }

    #[test]
    fn movetext_from_a_black_to_move_position() {
        let mut g =
            Game::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap();
        g.play_uci("e7e5").unwrap();
        g.play_uci("g1f3").unwrap();
        assert_eq!(g.movetext(), "1... e5 2. Nf3");
    }
}
