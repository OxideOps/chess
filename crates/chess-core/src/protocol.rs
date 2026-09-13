//! Messages exchanged over the game WebSocket. Shared by the server and the
//! client so the two can never disagree about the wire format.
//!
//! Encoded as JSON with an external `"type"` tag, e.g.
//! `{"type":"move","uci":"e2e4"}`. This is a first draft: expect it to grow
//! (chat, rematch, spectators, takebacks) before the server ships.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use shakmaty::{Color, uci::UciMove};

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
pub struct Clocks {
    pub white_ms: u64,
    pub black_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Move { uci: UciMove },
    Resign,
    OfferDraw,
    AcceptDraw,
    DeclineDraw,
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// The complete game, sent on connect and reconnect.
    Sync {
        start_fen: String,
        moves: Vec<UciMove>,
        clocks: Clocks,
        /// `None` for spectators.
        #[serde(with = "opt_color")]
        your_color: Option<Color>,
    },
    /// A move was accepted (either side's). `ply` lets a client detect gaps.
    MovePlayed {
        ply: u32,
        uci: UciMove,
        clocks: Clocks,
    },
    DrawOffered {
        #[serde(with = "color")]
        by: Color,
    },
    GameOver {
        result: GameResult,
        reason: GameOverReason,
    },
    /// A request was rejected. The game state is unchanged.
    Rejected {
        message: String,
    },
    Pong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameResult {
    WhiteWins,
    BlackWins,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
            result: GameResult::Draw,
            reason: GameOverReason::Repetition,
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
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""your_color":"black""#), "{json}");
        assert_eq!(serde_json::from_str::<ServerMessage>(&json).unwrap(), msg);
        let spectator = json.replace(r#""black""#, "null");
        assert!(matches!(
            serde_json::from_str::<ServerMessage>(&spectator).unwrap(),
            ServerMessage::Sync {
                your_color: None,
                ..
            }
        ));
    }
}
