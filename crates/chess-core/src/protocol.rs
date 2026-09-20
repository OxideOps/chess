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
pub mod color {
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

/// Who sits on a side. `username` and `rating` are `None` for guests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct PlayerInfo {
    pub username: Option<String>,
    /// Their rating in the game's category when they sat down.
    #[serde(default)]
    pub rating: Option<PlayerRating>,
}

/// A rating as shown next to a name: rounded, and whether it's still a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct PlayerRating {
    pub value: i32,
    pub provisional: bool,
}

/// What a rated game did to each side's rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct RatingDiffs {
    pub white: i32,
    pub black: i32,
}

/// Both seats; `None` while a seat is still open.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Players {
    pub white: Option<PlayerInfo>,
    pub black: Option<PlayerInfo>,
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
        #[serde(default)]
        players: Players,
        /// A player who has left and is counting down to losing the game.
        #[serde(default)]
        away: Option<Away>,
        /// Whether the result changes ratings.
        #[serde(default)]
        rated: bool,
        category: Category,
        /// Set once a rated game's ratings have been updated.
        #[serde(default)]
        rating_diffs: Option<RatingDiffs>,
    },
    /// A rated game's ratings were updated (after `GameOver`).
    RatingsChanged {
        #[serde(flatten)]
        diffs: RatingDiffs,
    },
    /// Someone's reconnect countdown started or stopped.
    AwayChanged {
        away: Option<Away>,
    },
    /// A seat was taken (or, later, vacated).
    PlayersChanged {
        players: Players,
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

/// An open offer to play, as shown in the lobby. A seek is not a game: the
/// game is only created when someone accepts, so nobody sits at a
/// half-empty board waiting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct SeekInfo {
    pub id: String,
    /// Who posted it; `None` for a guest.
    pub username: Option<String>,
    /// Their rating in this seek's category; `None` for a guest.
    pub rating: Option<PlayerRating>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub initial_ms: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub increment_ms: u64,
    pub rated: bool,
    pub category: Category,
}

/// Messages a client sends on the lobby socket (`/api/lobby/ws`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum LobbyClientMessage {
    /// Offer a game. Replaces this connection's previous seek, if any: one
    /// person waiting is one line in the list.
    PostSeek {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        initial_ms: u64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        increment_ms: u64,
        rated: bool,
    },
    /// Withdraw this connection's seek.
    CancelSeek,
    /// Take someone else's seek. At most one accept can win.
    AcceptSeek {
        id: String,
    },
    Ping,
}

/// Messages the lobby socket sends back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum LobbyServerMessage {
    /// Every open seek. Sent on connect and whenever the list changes; the
    /// list is small enough that sending all of it beats sending deltas.
    Seeks {
        seeks: Vec<SeekInfo>,
    },
    /// The id of the seek this connection just posted, so a client can tell
    /// its own from everyone else's.
    SeekPosted {
        id: String,
    },
    /// A seek involving this connection's user was taken: go and play.
    GameStarted {
        game_id: String,
        #[serde(with = "color")]
        #[cfg_attr(feature = "ts", ts(type = "\"white\" | \"black\""))]
        your_color: Color,
    },
    /// A request was refused. Nothing changed.
    Rejected {
        message: String,
    },
    Pong,
}

/// A seated player with no connection while their opponent is there. If
/// they don't come back within `ms`, the game ends: aborted if both sides
/// hadn't moved yet, otherwise lost by abandonment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub struct Away {
    #[serde(with = "color")]
    #[cfg_attr(feature = "ts", ts(type = "\"white\" | \"black\""))]
    pub side: Color,
    /// Milliseconds left to reconnect, as of when the message was sent.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub ms: u64,
}

/// A game's speed, from its estimated duration: the initial time plus 40
/// increments (Lichess's convention). Ratings are kept per category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(TS), ts(export))]
pub enum Category {
    Bullet,
    Blitz,
    Rapid,
    Classical,
}

impl Category {
    pub const ALL: [Category; 4] = [
        Category::Bullet,
        Category::Blitz,
        Category::Rapid,
        Category::Classical,
    ];

    pub fn of(initial_ms: u64, increment_ms: u64) -> Category {
        let estimate_s = (initial_ms + 40 * increment_ms) / 1000;
        match estimate_s {
            0..180 => Category::Bullet,
            180..480 => Category::Blitz,
            480..1500 => Category::Rapid,
            _ => Category::Classical,
        }
    }

    /// The name used in the database and on the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Bullet => "bullet",
            Category::Blitz => "blitz",
            Category::Rapid => "rapid",
            Category::Classical => "classical",
        }
    }

    pub fn parse(s: &str) -> Option<Category> {
        Category::ALL.into_iter().find(|c| c.as_str() == s)
    }
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
    /// No result: the game was called off before both sides had moved.
    Aborted,
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
            players: Players::default(),
            away: Some(Away {
                side: Color::White,
                ms: 42_000,
            }),
            rated: true,
            category: Category::Blitz,
            rating_diffs: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(
            json.contains(r#""away":{"side":"white","ms":42000}"#),
            "{json}"
        );
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
    fn categories_follow_the_estimated_duration() {
        let of = |min: u64, inc_s: u64| Category::of(min * 60_000, inc_s * 1000);
        assert_eq!(of(1, 0), Category::Bullet);
        assert_eq!(of(2, 1), Category::Bullet); // 120 s + 40 × 1 s = 160 s
        assert_eq!(of(3, 0), Category::Blitz);
        assert_eq!(of(3, 2), Category::Blitz);
        assert_eq!(of(5, 0), Category::Blitz);
        assert_eq!(of(8, 0), Category::Rapid);
        assert_eq!(of(10, 0), Category::Rapid);
        assert_eq!(of(15, 10), Category::Rapid); // 900 s + 400 s
        assert_eq!(of(20, 10), Category::Classical); // 1200 s + 400 s
        assert_eq!(of(25, 0), Category::Classical);
        for c in Category::ALL {
            assert_eq!(Category::parse(c.as_str()), Some(c));
            assert_eq!(
                serde_json::to_string(&c).unwrap(),
                format!("\"{}\"", c.as_str())
            );
        }
    }

    #[test]
    fn away_and_aborted_on_the_wire() {
        let json = serde_json::to_string(&ServerMessage::AwayChanged { away: None }).unwrap();
        assert_eq!(json, r#"{"type":"away_changed","away":null}"#);
        let end = ServerMessage::GameOver {
            end: GameEnd {
                result: GameResult::Aborted,
                reason: GameOverReason::Abandoned,
            },
        };
        assert_eq!(
            serde_json::to_string(&end).unwrap(),
            r#"{"type":"game_over","result":"aborted","reason":"abandoned"}"#
        );
        // Optional fields may be left out.
        let old = r#"{"type":"sync","start_fen":"f","moves":[],"clocks":{"white_ms":1,"black_ms":1},"your_color":null,"category":"rapid"}"#;
        assert!(matches!(
            serde_json::from_str::<ServerMessage>(old).unwrap(),
            ServerMessage::Sync {
                away: None,
                rated: false,
                rating_diffs: None,
                ..
            }
        ));
        let json = serde_json::to_string(&ServerMessage::RatingsChanged {
            diffs: RatingDiffs {
                white: 12,
                black: -12,
            },
        })
        .unwrap();
        assert_eq!(json, r#"{"type":"ratings_changed","white":12,"black":-12}"#);
        let player: PlayerInfo = serde_json::from_str(r#"{"username":"dan"}"#).unwrap();
        assert_eq!(player.rating, None);
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
