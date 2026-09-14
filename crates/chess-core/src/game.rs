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

/// Identifies a move (a node of the move tree) within one [`Game`]. The root,
/// the start position, is `0`. Ids stay valid until the node is deleted.
pub type NodeId = usize;

#[derive(Debug, Clone)]
struct Node {
    position: Chess,
    /// The move that led here; `None` for the root.
    played: Option<PlayedMove>,
    parent: Option<NodeId>,
    /// The first child is the main continuation; the rest are variations.
    children: Vec<NodeId>,
    /// `false` once deleted (ids are never reused).
    alive: bool,
}

/// One element of the game written out as PGN movetext: a move, or the start
/// or end of a variation. See [`Game::tokens`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Move {
        id: NodeId,
        /// `12.` before a White move, `12...` where a Black move needs one
        /// (the first move of a line, or right after a variation).
        number: Option<String>,
        san: String,
        /// How many variations deep: 0 on the main line.
        depth: u32,
    },
    VariationStart,
    VariationEnd,
}

/// A chess game with variations: a start position and a tree of moves from
/// it, of which one line (from the start to the end of some branch) is
/// current, with a cursor on it.
///
/// The linear API (`moves`, `ply_count`, `cursor`, `go_back`, …) works on the
/// current line, so code that never makes variations sees a plain game
/// record. [`Game::play`] appends to the end of the current line;
/// [`Game::play_here_from_to`] and [`Game::play_here_uci`] play from the
/// cursor, following the move if it's already there and otherwise starting a
/// variation, which becomes the current line.
#[derive(Debug, Clone)]
pub struct Game {
    start_fen: Fen,
    nodes: Vec<Node>,
    /// The current line: node ids from the root to the end of a branch.
    line: Vec<NodeId>,
    /// Index into `line` of the node being viewed.
    cursor: usize,
    /// The moves along `line`, for [`Game::moves`].
    line_moves: Vec<PlayedMove>,
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

    /// Load the first game in a PGN, with its variations. See [`crate::pgn`].
    pub fn from_pgn(pgn: &str) -> Result<Self, GameError> {
        crate::pgn::parse(pgn).map(|p| p.game)
    }

    fn from_position(pos: Chess) -> Self {
        Self {
            start_fen: Fen::from_position(&pos, EnPassantMode::Legal),
            nodes: vec![Node {
                position: pos,
                played: None,
                parent: None,
                children: Vec::new(),
                alive: true,
            }],
            line: vec![0],
            cursor: 0,
            line_moves: Vec::new(),
        }
    }

    // ----- the tree ------------------------------------------------------

    fn node_at(&self, ply: usize) -> &Node {
        &self.nodes[self.line[ply]]
    }

    fn live_children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes[id]
            .children
            .iter()
            .copied()
            .filter(|&c| self.nodes[c].alive)
    }

    /// Make the current line run from the root through `id` and on along
    /// main continuations, with the cursor on `id`.
    fn select(&mut self, id: NodeId) {
        let mut path = vec![id];
        let mut up = id;
        while let Some(parent) = self.nodes[up].parent {
            path.push(parent);
            up = parent;
        }
        path.reverse();
        self.cursor = path.len() - 1;
        let mut down = id;
        while let Some(next) = self.live_children(down).next() {
            path.push(next);
            down = next;
        }
        self.line = path;
        self.line_moves = self.line[1..]
            .iter()
            .map(|&n| {
                self.nodes[n]
                    .played
                    .clone()
                    .expect("only the root has no move")
            })
            .collect();
    }

    /// Add `mv` (legal in `parent`'s position) as a child of `parent`, or
    /// find the child that already plays it.
    fn child(&mut self, parent: NodeId, mv: Move) -> Result<NodeId, GameError> {
        let pos = self.nodes[parent].position.clone();
        if !pos.is_legal(mv) {
            return Err(GameError::IllegalMove);
        }
        if let Some(existing) = self
            .live_children(parent)
            .find(|&c| self.nodes[c].played.as_ref().is_some_and(|p| p.mv == mv))
        {
            return Ok(existing);
        }
        let uci = UciMove::from_move(mv, CastlingMode::Standard);
        let san = SanPlus::from_move(pos.clone(), mv);
        let next = pos.play(mv).map_err(|_| GameError::IllegalMove)?;
        let id = self.nodes.len();
        self.nodes.push(Node {
            position: next,
            played: Some(PlayedMove { mv, san, uci }),
            parent: Some(parent),
            children: Vec::new(),
            alive: true,
        });
        self.nodes[parent].children.push(id);
        Ok(id)
    }

    /// The nodes of the current line, from the start (`0`) to its end.
    pub fn line(&self) -> &[NodeId] {
        &self.line
    }

    /// The node at the cursor.
    pub fn node(&self) -> NodeId {
        self.line[self.cursor]
    }

    /// The move before `id`; `None` for the start (and unknown ids).
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes
            .get(id)
            .filter(|n| n.alive)
            .and_then(|n| n.parent)
    }

    /// Whether `id` names a move (or the start) that hasn't been deleted.
    pub fn has_node(&self, id: NodeId) -> bool {
        self.nodes.get(id).is_some_and(|n| n.alive)
    }

    /// Whether `id` is on the main line (every move on the way is a main
    /// continuation).
    pub fn is_main_line(&self, id: NodeId) -> bool {
        let mut node = id;
        while let Some(parent) = self.nodes[node].parent {
            if self.live_children(parent).next() != Some(node) {
                return false;
            }
            node = parent;
        }
        true
    }

    /// View `id`: the current line becomes the one through it. Unknown or
    /// deleted ids change nothing.
    pub fn go_to_node(&mut self, id: NodeId) {
        if self.has_node(id) {
            self.select(id);
        }
    }

    /// Switch to a sibling of the move at the cursor (the next variation for
    /// `+1`, the previous for `-1`), keeping to the same ply. Nothing happens
    /// at the start or when the move has no alternatives.
    pub fn switch_variation(&mut self, step: i32) {
        let Some(parent) = self.nodes[self.node()].parent else {
            return;
        };
        let siblings: Vec<NodeId> = self.live_children(parent).collect();
        let Some(i) = siblings.iter().position(|&s| s == self.node()) else {
            return;
        };
        let j = i as i64 + i64::from(step);
        if let Some(&target) = usize::try_from(j).ok().and_then(|j| siblings.get(j)) {
            self.select(target);
        }
    }

    /// Promote the variation that `id` is in one level: at the branch point
    /// nearest to `id`, its line becomes the main continuation. Returns
    /// whether anything changed (it doesn't on the main line).
    pub fn promote(&mut self, id: NodeId) -> bool {
        if !self.has_node(id) {
            return false;
        }
        let mut node = id;
        while let Some(parent) = self.nodes[node].parent {
            if self.live_children(parent).next() != Some(node) {
                let children = &mut self.nodes[parent].children;
                children.retain(|&c| c != node);
                children.insert(0, node);
                let end = *self.line.last().expect("never empty");
                let here = self.node();
                // Keep viewing the same move and the same branch.
                self.select(end);
                self.select_keeping_line(here);
                return true;
            }
            node = parent;
        }
        false
    }

    /// Delete `id` and everything after it. If it was on the current line,
    /// the cursor moves to the move before it. The start can't be deleted.
    pub fn delete_from(&mut self, id: NodeId) -> bool {
        let Some(parent) = self
            .nodes
            .get(id)
            .filter(|n| n.alive)
            .and_then(|n| n.parent)
        else {
            return false;
        };
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            self.nodes[n].alive = false;
            stack.extend(self.nodes[n].children.iter().copied());
        }
        self.nodes[parent].children.retain(|&c| c != id);
        let here = self.node();
        if self.has_node(here) {
            let end = *self.line.last().expect("never empty");
            if self.has_node(end) {
                self.select_keeping_line(here);
            } else {
                self.select(here);
            }
        } else {
            self.select(parent);
        }
        true
    }

    /// Put the cursor on `id` if it's on the current line; otherwise select it.
    fn select_keeping_line(&mut self, id: NodeId) {
        match self.line.iter().position(|&n| n == id) {
            Some(i) => {
                let end = *self.line.last().expect("never empty");
                self.select(end);
                self.cursor = i;
            }
            None => self.select(id),
        }
    }

    /// The whole tree as movetext tokens: the main line with each
    /// variation, in brackets, right after the move it replaces, and moves
    /// numbered the way PGN does. [`Game::movetext`] is these, joined.
    pub fn tokens(&self) -> Vec<Token> {
        let mut out = Vec::new();
        self.write_tokens(0, 0, true, &mut out);
        out
    }

    fn move_token(&self, id: NodeId, depth: u32, force_number: bool) -> Token {
        let node = &self.nodes[id];
        let before = &self.nodes[node.parent.expect("a move has a parent")].position;
        let n = before.fullmoves().get();
        let number = if before.turn() == Color::White {
            Some(format!("{n}."))
        } else if force_number {
            Some(format!("{n}..."))
        } else {
            None
        };
        Token::Move {
            id,
            number,
            san: node.played.as_ref().expect("a move").san.to_string(),
            depth,
        }
    }

    fn write_tokens(&self, from: NodeId, depth: u32, force_number: bool, out: &mut Vec<Token>) {
        let mut parent = from;
        let mut force = force_number;
        loop {
            let children: Vec<NodeId> = self.live_children(parent).collect();
            let Some((&main, variations)) = children.split_first() else {
                break;
            };
            out.push(self.move_token(main, depth, force));
            force = false;
            for &alt in variations {
                out.push(Token::VariationStart);
                out.push(self.move_token(alt, depth + 1, true));
                self.write_tokens(alt, depth + 1, false, out);
                out.push(Token::VariationEnd);
                force = true;
            }
            parent = main;
        }
    }

    // ----- reading state -------------------------------------------------

    /// The position at the cursor.
    pub fn position(&self) -> &Chess {
        &self.node_at(self.cursor).position
    }

    /// The position the game started from.
    pub fn start_position(&self) -> &Chess {
        &self.nodes[0].position
    }

    /// The position at the end of the current line, ignoring the cursor.
    pub fn latest(&self) -> &Chess {
        &self.node_at(self.line.len() - 1).position
    }

    pub fn start_fen(&self) -> String {
        self.start_fen.to_string()
    }

    /// FEN of the position at the cursor.
    pub fn fen(&self) -> String {
        Fen::from_position(self.position(), EnPassantMode::Legal).to_string()
    }

    /// The moves of the current line.
    pub fn moves(&self) -> &[PlayedMove] {
        &self.line_moves
    }

    /// Number of half-moves on the current line (not counting anything
    /// before the start position).
    pub fn ply_count(&self) -> usize {
        self.line.len() - 1
    }

    /// Number of half-moves from the start to the cursor.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_viewing_history(&self) -> bool {
        self.cursor != self.ply_count()
    }

    /// The move that led to the position at the cursor, if any.
    pub fn last_move(&self) -> Option<&PlayedMove> {
        self.node_at(self.cursor).played.as_ref()
    }

    pub fn turn(&self) -> Color {
        self.position().turn()
    }

    /// Status of the position at the cursor.
    pub fn status(&self) -> GameStatus {
        self.status_at(self.cursor)
    }

    /// Status of the position at the end of the current line, ignoring the cursor.
    pub fn final_status(&self) -> GameStatus {
        self.status_at(self.ply_count())
    }

    fn status_at(&self, ply: usize) -> GameStatus {
        let pos = &self.node_at(ply).position;
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

    /// How many times the position at the cursor has occurred so far on the
    /// current line (including this occurrence).
    pub fn repetition_count(&self) -> usize {
        self.repetition_count_at(self.cursor)
    }

    fn repetition_count_at(&self, ply: usize) -> usize {
        let hash = self
            .node_at(ply)
            .position
            .zobrist_hash::<Zobrist64>(EnPassantMode::Legal);
        self.line[..=ply]
            .iter()
            .filter(|&&n| {
                self.nodes[n]
                    .position
                    .zobrist_hash::<Zobrist64>(EnPassantMode::Legal)
                    == hash
            })
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

    /// Play a fully specified move at the end of the current line.
    pub fn play(&mut self, mv: Move) -> Result<&PlayedMove, GameError> {
        let end = *self.line.last().expect("never empty");
        let id = self.child(end, mv)?;
        self.select(id);
        Ok(self.last_move().expect("just played"))
    }

    /// Play a move from the position at the cursor: follow it if it's
    /// already there, otherwise start a variation with it. Either way the
    /// line through it becomes current.
    fn play_here(&mut self, mv: Move) -> Result<&PlayedMove, GameError> {
        let id = self.child(self.node(), mv)?;
        self.select(id);
        Ok(self.last_move().expect("just played"))
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

    /// Like [`Game::play_from_to`], but from the position at the cursor: a
    /// move already played there is followed, a new one starts a variation.
    /// Nothing changes if the move is illegal or still needs a promotion piece.
    pub fn play_here_from_to(
        &mut self,
        from: Square,
        to: Square,
        promotion: Option<Role>,
    ) -> Result<&PlayedMove, GameError> {
        let mv = resolve_move(self.position(), from, to, promotion)?;
        self.play_here(mv)
    }

    /// Play a UCI move from the position at the cursor, as
    /// [`Game::play_here_from_to`] does. Used to follow an engine line.
    pub fn play_here_uci(&mut self, uci: &str) -> Result<&PlayedMove, GameError> {
        let parsed: UciMove = uci
            .parse()
            .map_err(|_| GameError::InvalidUci(uci.to_string()))?;
        let mv = parsed
            .to_move(self.position())
            .map_err(|_| GameError::IllegalMove)?;
        self.play_here(mv)
    }

    /// Play a SAN move from the position at the cursor, as
    /// [`Game::play_here_from_to`] does. Used by the PGN reader.
    pub fn play_here_san(&mut self, san: &SanPlus) -> Result<&PlayedMove, GameError> {
        let mv = san
            .san
            .to_move(self.position())
            .map_err(|_| GameError::IllegalMove)?;
        self.play_here(mv)
    }

    /// Play a move in UCI notation (`e2e4`, `e7e8q`) at the end of the
    /// current line, e.g. from the wire or an engine.
    pub fn play_uci(&mut self, uci: &str) -> Result<&PlayedMove, GameError> {
        let parsed: UciMove = uci
            .parse()
            .map_err(|_| GameError::InvalidUci(uci.to_string()))?;
        let mv = parsed
            .to_move(self.latest())
            .map_err(|_| GameError::IllegalMove)?;
        self.play(mv)
    }

    /// Forget every move after the cursor, variations included, so the
    /// viewed position becomes the end of the game.
    pub fn truncate_to_cursor(&mut self) {
        let here = self.node();
        let children: Vec<NodeId> = self.live_children(here).collect();
        for child in children {
            self.delete_from(child);
        }
        self.select(here);
    }

    // ----- navigation ----------------------------------------------------

    pub fn go_to_ply(&mut self, ply: usize) {
        self.cursor = ply.min(self.ply_count());
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
        self.cursor = self.ply_count();
    }

    // ----- export --------------------------------------------------------

    /// PGN movetext without headers or result, variations included, e.g.
    /// `1. e4 e5 (1... c5 2. Nf3) 2. Nf3`. Starts from the game's start
    /// position, so a game that began from a FEN with Black to move opens
    /// with `1... e5`.
    pub fn movetext(&self) -> String {
        let mut out = String::new();
        let mut after_open = false;
        for token in self.tokens() {
            match token {
                Token::Move { number, san, .. } => {
                    if !out.is_empty() && !after_open {
                        out.push(' ');
                    }
                    if let Some(n) = number {
                        let _ = write!(out, "{n} ");
                    }
                    out.push_str(&san);
                    after_open = false;
                }
                Token::VariationStart => {
                    out.push_str(" (");
                    after_open = true;
                }
                Token::VariationEnd => out.push(')'),
            }
        }
        out
    }

    /// The result token for the game as played along the main line: `1-0`,
    /// `0-1`, `1/2-1/2`, or `*` while it is unfinished.
    pub fn result_token(&self) -> &'static str {
        let mut end = 0;
        while let Some(next) = self.live_children(end).next() {
            end = next;
        }
        let mut main = self.clone();
        main.select(end);
        let status = main.final_status();
        match status.winner() {
            Some(Color::White) => "1-0",
            Some(Color::Black) => "0-1",
            None if status.is_game_over() => "1/2-1/2",
            None => "*",
        }
    }

    /// A minimal PGN export: `SetUp`/`FEN` tags when the game didn't start
    /// from the initial position, the movetext with variations, and the
    /// result token.
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
    fn playing_from_history_starts_a_variation() {
        let mut g = game("e2e4 e7e5 g1f3 b8c6");
        g.go_to_ply(1);
        // An illegal move changes nothing, not even the history.
        assert_eq!(
            g.play_here_from_to(sq("e7"), sq("e4"), None),
            Err(GameError::IllegalMove)
        );
        assert_eq!(g.ply_count(), 4);
        assert_eq!(g.cursor(), 1);

        // A new move keeps the old ones as the main line and becomes current.
        g.play_here_from_to(sq("c7"), sq("c5"), None).unwrap();
        assert_eq!(g.movetext(), "1. e4 e5 (1... c5) 2. Nf3 Nc6");
        assert_eq!(uci_line(&g), ["e2e4", "c7c5"]);
        assert!(!g.is_viewing_history());
        assert!(!g.is_main_line(g.node()));

        // From the start, a second first move; the main line resumes with a number.
        g.go_to_start();
        g.play_here_uci("d2d4").unwrap();
        assert_eq!(g.movetext(), "1. e4 (1. d4) 1... e5 (1... c5) 2. Nf3 Nc6");
        assert_eq!(g.play_here_uci("d2d4"), Err(GameError::IllegalMove));

        // Playing a move that's already there follows it instead of copying it.
        g.go_to_start();
        g.play_here_uci("e2e4").unwrap();
        assert_eq!(g.movetext(), "1. e4 (1. d4) 1... e5 (1... c5) 2. Nf3 Nc6");
        assert_eq!(uci_line(&g), ["e2e4", "e7e5", "g1f3", "b8c6"]);
        assert_eq!(g.cursor(), 1);
    }

    fn uci_line(g: &Game) -> Vec<String> {
        g.moves().iter().map(|m| m.uci.to_string()).collect()
    }

    /// The node on the current line `ply` half-moves in.
    fn node_at_ply(g: &mut Game, ply: usize) -> NodeId {
        g.go_to_ply(ply);
        g.node()
    }

    #[test]
    fn the_current_line_remembers_its_branch() {
        let mut g = game("e2e4 e7e5 g1f3 b8c6");
        g.go_to_ply(1);
        g.play_here_uci("c7c5").unwrap();
        g.play_here_uci("g1f3").unwrap();
        g.play_here_uci("d7d6").unwrap();
        // Back and forward stay inside the Sicilian.
        g.go_to_ply(1);
        g.go_forward();
        g.go_forward();
        assert_eq!(g.last_move().unwrap().uci.to_string(), "g1f3");
        g.go_to_end();
        assert_eq!(uci_line(&g), ["e2e4", "c7c5", "g1f3", "d7d6"]);
        // Clicking a main-line move puts the main line back.
        let c6 = {
            let mut main = g.clone();
            main.go_to_node(0);
            node_at_ply(&mut main, 4)
        };
        g.go_to_node(c6);
        assert_eq!(uci_line(&g), ["e2e4", "e7e5", "g1f3", "b8c6"]);
        assert_eq!(g.cursor(), 4);
        assert!(g.is_main_line(c6));
        // Unknown ids change nothing.
        g.go_to_node(9999);
        assert_eq!(g.cursor(), 4);
    }

    #[test]
    fn switching_between_variations() {
        let mut g = game("e2e4 e7e5");
        g.go_to_ply(1);
        g.play_here_uci("c7c5").unwrap();
        g.go_to_ply(1);
        g.play_here_uci("e7e6").unwrap();
        // Siblings of ...e6, in order: e5 (main), c5, e6.
        g.switch_variation(-1);
        assert_eq!(g.last_move().unwrap().uci.to_string(), "c7c5");
        g.switch_variation(-1);
        assert_eq!(g.last_move().unwrap().uci.to_string(), "e7e5");
        g.switch_variation(-1); // already the first: stays
        assert_eq!(g.last_move().unwrap().uci.to_string(), "e7e5");
        g.switch_variation(2);
        assert_eq!(g.last_move().unwrap().uci.to_string(), "e7e6");
        // A move without alternatives, and the start, don't move.
        g.go_to_start();
        g.switch_variation(1);
        assert_eq!(g.cursor(), 0);
    }

    #[test]
    fn promoting_and_deleting_variations() {
        let mut g = game("e2e4 e7e5 g1f3");
        g.go_to_ply(1);
        g.play_here_uci("c7c5").unwrap();
        g.play_here_uci("g1f3").unwrap();
        let sicilian_nf3 = g.node();
        assert_eq!(g.movetext(), "1. e4 e5 (1... c5 2. Nf3) 2. Nf3");

        // Promoting from anywhere in the variation makes it the main line,
        // and keeps viewing the same move.
        assert!(g.promote(sicilian_nf3));
        assert_eq!(g.movetext(), "1. e4 c5 (1... e5 2. Nf3) 2. Nf3");
        assert_eq!(g.node(), sicilian_nf3);
        assert!(g.is_main_line(sicilian_nf3));
        assert!(!g.promote(sicilian_nf3)); // nothing above it to promote
        assert_eq!(g.result_token(), "*");

        // Deleting the move at the cursor goes back to the move before it.
        assert!(g.delete_from(sicilian_nf3));
        assert_eq!(g.movetext(), "1. e4 c5 (1... e5 2. Nf3)");
        assert_eq!(uci_line(&g), ["e2e4", "c7c5"]);
        assert!(!g.has_node(sicilian_nf3));
        assert!(!g.delete_from(sicilian_nf3));
        // Deleting a variation elsewhere keeps the view.
        let e5 = {
            let mut other = g.clone();
            other.switch_variation(1);
            other.node()
        };
        assert!(g.delete_from(e5));
        assert_eq!(g.movetext(), "1. e4 c5");
        assert_eq!(uci_line(&g), ["e2e4", "c7c5"]);
        assert_eq!(g.cursor(), 2);
        assert!(!g.delete_from(0)); // not the start
    }

    #[test]
    fn tokens_number_moves_and_mark_depth() {
        let mut g = game("e2e4 e7e5 g1f3");
        g.go_to_ply(2);
        g.play_here_uci("f1c4").unwrap();
        g.go_to_ply(2);
        g.play_here_uci("b1c3").unwrap();
        let tokens = g.tokens();
        let text: Vec<String> = tokens
            .iter()
            .map(|t| match t {
                Token::Move {
                    number, san, depth, ..
                } => format!(
                    "{}{san}@{depth}",
                    number.clone().map(|n| n + " ").unwrap_or_default()
                ),
                Token::VariationStart => "(".into(),
                Token::VariationEnd => ")".into(),
            })
            .collect();
        assert_eq!(
            text,
            [
                "1. e4@0", "e5@0", "2. Nf3@0", "(", "2. Bc4@1", ")", "(", "2. Nc3@1", ")"
            ]
        );
        assert_eq!(g.movetext(), "1. e4 e5 2. Nf3 (2. Bc4) (2. Nc3)");
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
