//! Messages exchanged over the game WebSocket. Shared by the server and the
//! client so the two can never disagree about the wire format.
//!
//! Encoded as JSON with an external `"type"` tag, e.g.
//! `{"type":"move","uci":"e2e4"}`. This is a first draft: expect it to grow
//! (chat, rematch, spectators, takebacks) before the server ships.
//!
//! With the `ts` feature, `cargo test --features ts` writes TypeScript
//! bindings for these types (see `web/scripts/gen-types.mjs`), so the
//! SvelteKit client can't drift from the server either.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use shakmaty::{Color, uci::UciMove};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::GameStatus;

/// `shakmaty::Color` has no serde support; encode it as `"white"` / `"black"`.
mod color {
    use super::*;

    pub fn serialize<S: Serializer>(color: &Color, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(if color.is_white() { "white" } else { "black" })
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Color, D::Error> {
        match <&str>::deserialize(d)? {
            "white" => Ok(Color::White),
            "black" => Ok(Color::Black),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["white", "black"],
            )),
        }
    }
}

mod opt_color {
    use super::*;

    pub fn serialize<S: Serializer>(color: &Option<Color>, s: S) -> Result<S::Ok, S::Error> {
        match color {
            Some(c) => color::serialize(c, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Color>, D::Error> {
        Option::<&str>::deserialize(d)?
            .map(|s| match s {
                "white" => Ok(Color::White),
                "black" => Ok(Color::Black),
                other => Err(serde::de::Error::unknown_variant(
                    other,
                    &["white", "black"],
                )),
            })
            .transpose()
    }
}

/// Remaining time for both sides, in milliseconds. The server owns the clocks;
/// clients only display them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Clocks {
    // Milliseconds fit in a JS number for any game we'll ever host.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub white_ms: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub black_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum ClientMessage {
    Move {
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        uci: UciMove,
    },
    Resign,
    OfferDraw,
    AcceptDraw,
    DeclineDraw,
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum ServerMessage {
    /// The complete game, sent on connect and reconnect.
    Sync {
        start_fen: String,
        #[cfg_attr(feature = "ts", ts(type = "string[]"))]
        moves: Vec<UciMove>,
        clocks: Clocks,
        /// `None` for spectators.
        #[serde(with = "opt_color")]
        #[cfg_attr(feature = "ts", ts(type = "\"white\" | \"black\" | null"))]
        your_color: Option<Color>,
        /// Set once the game is over.
        #[serde(default)]
        ended: Option<GameEnd>,
        /// A draw offer that is still open.
        #[serde(default, with = "opt_color")]
        #[cfg_attr(feature = "ts", ts(type = "\"white\" | \"black\" | null"))]
        draw_offer: Option<Color>,
    },
    /// A move was accepted (either side's). `ply` lets a client detect gaps.
    MovePlayed {
        ply: u32,
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        uci: UciMove,
        clocks: Clocks,
    },
    DrawOffered {
        #[serde(with = "color")]
        #[cfg_attr(feature = "ts", ts(type = "\"white\" | \"black\""))]
        by: Color,
    },
    DrawDeclined,
    GameOver {
        #[serde(flatten)]
        end: GameEnd,
    },
    /// A request was rejected. The game state is unchanged.
    Rejected {
        message: String,
    },
    Pong,
}

/// How a game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct GameEnd {
    pub result: GameResult,
    pub reason: GameOverReason,
}

impl GameEnd {
    /// The end implied by the rules for a position, if it is over.
    pub fn from_status(status: GameStatus) -> Option<GameEnd> {
        let reason = match status {
            GameStatus::Ongoing | GameStatus::Check => return None,
            GameStatus::Checkmate { .. } => GameOverReason::Checkmate,
            GameStatus::Stalemate => GameOverReason::Stalemate,
            GameStatus::InsufficientMaterial => GameOverReason::InsufficientMaterial,
            GameStatus::FiftyMoveRule => GameOverReason::FiftyMoves,
            GameStatus::ThreefoldRepetition => GameOverReason::Repetition,
        };
        Some(GameEnd {
            result: GameResult::from_winner(status.winner()),
            reason,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum GameResult {
    WhiteWins,
    BlackWins,
    Draw,
}

impl GameResult {
    pub fn from_winner(winner: Option<Color>) -> GameResult {
        match winner {
            Some(Color::White) => GameResult::WhiteWins,
            Some(Color::Black) => GameResult::BlackWins,
            None => GameResult::Draw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum GameOverReason {
    Checkmate,
    Resignation,
    Timeout,
    Stalemate,
    InsufficientMaterial,
    Agreement,
    Repetition,
    FiftyMoves,
    Abandoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_shape_is_stable() {
        let msg = ClientMessage::Move {
            uci: "e7e8q".parse().unwrap(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"move","uci":"e7e8q"}"#);
        assert_eq!(serde_json::from_str::<ClientMessage>(&json).unwrap(), msg);

        let msg = ServerMessage::GameOver {
            end: GameEnd {
                result: GameResult::Draw,
                reason: GameOverReason::Repetition,
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(
            json,
            r#"{"type":"game_over","result":"draw","reason":"repetition"}"#
        );
        assert_eq!(serde_json::from_str::<ServerMessage>(&json).unwrap(), msg);

        let msg = ServerMessage::Sync {
            start_fen: "fen".into(),
            moves: vec!["e2e4".parse().unwrap()],
            clocks: Clocks {
                white_ms: 1,
                black_ms: 2,
            },
            your_color: Some(Color::Black),
            ended: None,
            draw_offer: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""your_color":"black""#), "{json}");
        assert!(json.contains(r#""ended":null"#), "{json}");
        assert_eq!(serde_json::from_str::<ServerMessage>(&json).unwrap(), msg);
        let spectator = json.replace(r#""your_color":"black""#, r#""your_color":null"#);
        assert!(matches!(
            serde_json::from_str::<ServerMessage>(&spectator).unwrap(),
            ServerMessage::Sync {
                your_color: None,
                ..
            }
        ));
    }

    #[test]
    fn game_end_follows_the_rules() {
        use crate::Game;
        let mut g = Game::new();
        assert_eq!(GameEnd::from_status(g.status()), None);
        for m in ["e2e4", "e7e5", "f1c4", "b8c6", "d1h5", "g8f6", "h5f7"] {
            g.play_uci(m).unwrap();
        }
        assert_eq!(
            GameEnd::from_status(g.status()),
            Some(GameEnd {
                result: GameResult::WhiteWins,
                reason: GameOverReason::Checkmate
            })
        );
        let g = Game::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        assert_eq!(
            GameEnd::from_status(g.status()),
            Some(GameEnd {
                result: GameResult::Draw,
                reason: GameOverReason::Stalemate
            })
        );
    }
}
