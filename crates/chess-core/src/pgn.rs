//! A small PGN reader: enough to import a pasted game.
//!
//! Handles tag pairs, comments (`{...}` and `;`), nested variations (skipped),
//! NAGs (`$1`), `!?`-style suffixes, move numbers in any spacing, and result
//! tokens. Only the main line of the first game in the text is kept, which is
//! what an analysis board needs. Anything the rules reject is an error.

use shakmaty::{Position, san::SanPlus};

use crate::{Game, GameError};

/// A parsed game: its tag pairs in file order and the main line as a [`Game`].
#[derive(Debug, Clone)]
pub struct Pgn {
    pub headers: Vec<(String, String)>,
    pub game: Game,
}

impl Pgn {
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

const RESULTS: [&str; 4] = ["1-0", "0-1", "1/2-1/2", "*"];

/// Parse the first game in `text`.
pub fn parse(text: &str) -> Result<Pgn, GameError> {
    let mut p = Parser {
        src: text.as_bytes(),
        pos: 0,
    };
    let headers = p.headers()?;

    let mut game = match headers.iter().find(|(k, _)| k == "FEN") {
        Some((_, fen)) => Game::from_fen(fen)?,
        None => Game::new(),
    };
    if let Some((_, variant)) = headers.iter().find(|(k, _)| k == "Variant")
        && !matches!(
            variant.to_ascii_lowercase().as_str(),
            "standard" | "chess" | "from position"
        )
    {
        return Err(GameError::InvalidPgn(format!(
            "unsupported variant: {variant}"
        )));
    }

    p.movetext(&mut game)?;
    Ok(Pgn { headers, game })
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn error(&self, msg: impl Into<String>) -> GameError {
        GameError::InvalidPgn(msg.into())
    }

    fn headers(&mut self) -> Result<Vec<(String, String)>, GameError> {
        let mut headers = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'[') {
                return Ok(headers);
            }
            self.pos += 1;
            self.skip_whitespace();
            let start = self.pos;
            while self
                .peek()
                .is_some_and(|c| !c.is_ascii_whitespace() && c != b'"')
            {
                self.pos += 1;
            }
            let key = String::from_utf8_lossy(&self.src[start..self.pos]).into_owned();
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error(format!("tag pair [{key} ...] has no quoted value")));
            }
            self.pos += 1;
            let mut value = Vec::new();
            loop {
                match self.peek() {
                    None => return Err(self.error("unterminated tag value")),
                    Some(b'"') => break,
                    Some(b'\\') => {
                        self.pos += 1;
                        if let Some(c) = self.peek() {
                            value.push(c);
                        }
                    }
                    Some(c) => value.push(c),
                }
                self.pos += 1;
            }
            self.pos += 1; // closing quote
            self.skip_whitespace();
            if self.peek() != Some(b']') {
                return Err(self.error(format!("tag pair [{key} ...] is not closed")));
            }
            self.pos += 1;
            headers.push((key, String::from_utf8_lossy(&value).into_owned()));
        }
    }

    fn skip_brace_comment(&mut self) {
        // Called with the cursor on `{`.
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == b'}' {
                return;
            }
        }
    }

    fn skip_line(&mut self) {
        while self.peek().is_some_and(|c| c != b'\n') {
            self.pos += 1;
        }
    }

    fn skip_variation(&mut self) -> Result<(), GameError> {
        // Called with the cursor on `(`.
        let mut depth = 0usize;
        while let Some(c) = self.peek() {
            match c {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos += 1;
                        return Ok(());
                    }
                }
                b'{' => {
                    self.skip_brace_comment();
                    continue;
                }
                b';' => {
                    self.skip_line();
                    continue;
                }
                _ => {}
            }
            self.pos += 1;
        }
        Err(self.error("unterminated variation"))
    }

    fn movetext(&mut self, game: &mut Game) -> Result<(), GameError> {
        loop {
            self.skip_whitespace();
            let Some(c) = self.peek() else {
                return Ok(());
            };
            match c {
                b'{' => self.skip_brace_comment(),
                b';' => self.skip_line(),
                b'%' if self.pos == 0 || self.src[self.pos - 1] == b'\n' => self.skip_line(),
                b'(' => self.skip_variation()?,
                b')' => return Err(self.error("unexpected ')'")),
                b'$' => {
                    self.pos += 1;
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        self.pos += 1;
                    }
                }
                // Tag pairs after the movetext started belong to the next game.
                b'[' => return Ok(()),
                _ => {
                    let start = self.pos;
                    while self
                        .peek()
                        .is_some_and(|c| !c.is_ascii_whitespace() && !b"{};()[$".contains(&c))
                    {
                        self.pos += 1;
                    }
                    let token = &self.src[start..self.pos];
                    if RESULTS.iter().any(|r| r.as_bytes() == token) {
                        return Ok(());
                    }
                    if let Some(san) = san_token(token) {
                        self.play_san(game, san)?;
                    }
                }
            }
        }
    }

    fn play_san(&self, game: &mut Game, san: &[u8]) -> Result<(), GameError> {
        let text = String::from_utf8_lossy(san).into_owned();
        let san = SanPlus::from_ascii(san)
            .map_err(|_| self.error(format!("cannot read move \"{text}\"")))?;
        let mv = san.san.to_move(game.latest()).map_err(|_| {
            self.error(format!(
                "illegal move {text} at ply {} ({} to move)",
                game.ply_count() + 1,
                if game.latest().turn().is_white() {
                    "White"
                } else {
                    "Black"
                }
            ))
        })?;
        game.play(mv).map(|_| ())
    }
}

/// Strip a leading move number (`12.`, `12...`, `12.e4`) and trailing `!?`
/// annotations from a token. Returns `None` if nothing is left (a bare move
/// number or a stray `...`).
fn san_token(token: &[u8]) -> Option<&[u8]> {
    let mut start = 0;
    while token.get(start).is_some_and(|c| c.is_ascii_digit()) {
        start += 1;
    }
    if start > 0 || token.first() == Some(&b'.') {
        while token.get(start) == Some(&b'.') {
            start += 1;
        }
    } else {
        start = 0;
    }
    let mut end = token.len();
    while end > start && matches!(token[end - 1], b'!' | b'?') {
        end -= 1;
    }
    (end > start).then(|| &token[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameStatus;

    #[test]
    fn plain_movetext() {
        let pgn = parse("1. e4 e5 2. Nf3 Nc6 3. Bb5 a6").unwrap();
        assert!(pgn.headers.is_empty());
        assert_eq!(pgn.game.movetext(), "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6");
        assert_eq!(pgn.game.ply_count(), 6);
        assert!(!pgn.game.is_viewing_history());
    }

    #[test]
    fn headers_comments_variations_and_nags() {
        let text = r#"[Event "Casual game"]
[Site "?"]
[White "Someone \"quoted\""]
[Result "1-0"]

1. e4 {best by test} e5 2. Nf3 $1 (2. Bc4 Nf6 (2... Bc5 {also fine}) 3. d3) 2... Nc6
3. Bb5!? a6?! ; the Ruy Lopez
4.Ba4 Nf6 5.O-O 1-0"#;
        let pgn = parse(text).unwrap();
        assert_eq!(pgn.header("Event"), Some("Casual game"));
        assert_eq!(pgn.header("White"), Some("Someone \"quoted\""));
        assert_eq!(pgn.header("Result"), Some("1-0"));
        assert_eq!(
            pgn.game.movetext(),
            "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O"
        );
    }

    #[test]
    fn fen_header_sets_the_start_position() {
        let text = r#"[SetUp "1"]
[FEN "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"]

1... e5 2. Nf3 *"#;
        let pgn = parse(text).unwrap();
        assert_eq!(pgn.game.movetext(), "1... e5 2. Nf3");
        assert_eq!(pgn.game.start_position().turn(), shakmaty::Color::Black);
    }

    #[test]
    fn checkmate_and_promotion_notation() {
        let pgn = parse("1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6 4. Qxf7# 1-0").unwrap();
        assert!(matches!(pgn.game.status(), GameStatus::Checkmate { .. }));

        let pgn = parse("1. a4 b5 2. axb5 a6 3. bxa6 Nc6 4. a7 Nb8 5. axb8=N").unwrap();
        assert_eq!(pgn.game.moves().last().unwrap().san.to_string(), "axb8=N");
    }

    #[test]
    fn only_the_first_game_is_read() {
        let text = "[White \"A\"]\n\n1. d4 d5 1/2-1/2\n\n[White \"B\"]\n\n1. e4 *";
        let pgn = parse(text).unwrap();
        assert_eq!(pgn.game.movetext(), "1. d4 d5");

        // Also without a result token separating the games.
        let text = "1. d4 d5\n\n[White \"B\"]\n\n1. e4 *";
        assert_eq!(parse(text).unwrap().game.movetext(), "1. d4 d5");
    }

    #[test]
    fn errors_name_the_problem() {
        let err = parse("1. e4 e5 2. Ke2 Ke7 3. Nf6").unwrap_err();
        assert_eq!(
            err,
            GameError::InvalidPgn("illegal move Nf6 at ply 5 (White to move)".into())
        );
        assert!(matches!(parse("1. e4 (e5"), Err(GameError::InvalidPgn(_))));
        assert!(matches!(
            parse("[Event \"x\"\n1. e4"),
            Err(GameError::InvalidPgn(_))
        ));
        assert!(matches!(parse("1. e4 xyz"), Err(GameError::InvalidPgn(_))));
        assert!(matches!(
            parse("[Variant \"Crazyhouse\"]\n\n1. e4"),
            Err(GameError::InvalidPgn(_))
        ));
        assert!(matches!(
            parse("[FEN \"junk\"]\n\n*"),
            Err(GameError::InvalidFen(_))
        ));
    }

    #[test]
    fn empty_and_whitespace_input_is_an_empty_game() {
        assert_eq!(parse("").unwrap().game.ply_count(), 0);
        assert_eq!(parse("  \n *").unwrap().game.ply_count(), 0);
        assert_eq!(parse("[Event \"x\"]\n").unwrap().game.ply_count(), 0);
    }

    #[test]
    fn san_token_stripping() {
        assert_eq!(san_token(b"1."), None);
        assert_eq!(san_token(b"1..."), None);
        assert_eq!(san_token(b"..."), None);
        assert_eq!(san_token(b"12.e4"), Some(&b"e4"[..]));
        assert_eq!(san_token(b"12...Nf6!!"), Some(&b"Nf6"[..]));
        assert_eq!(san_token(b"O-O-O+"), Some(&b"O-O-O+"[..]));
        assert_eq!(san_token(b"e8=Q#?!"), Some(&b"e8=Q#"[..]));
    }
}
