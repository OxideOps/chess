//! Postgres persistence for games.
//!
//! The in-memory [`Room`](crate::room::Room) stays the source of truth while
//! the server runs; every change is written through as one row per game,
//! and a game that isn't in memory is loaded from here on first access.
//! Running without a database is allowed (nothing survives a restart then).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chess_core::{
    Color,
    protocol::{GameEnd, GameOverReason, GameResult},
};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    games::{GameListing, Seat},
    room::{Snapshot, TimeControl},
};

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

/// What we store per game besides the room itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredGame {
    pub seats: [Option<Seat>; 2],
    pub snapshot: Snapshot,
    /// How old the snapshot is: how long a running clock has kept running
    /// since it was written.
    pub age: Duration,
}

impl Db {
    /// Connect and apply migrations.
    pub async fn connect(url: &str) -> Result<Db, sqlx::Error> {
        let pool = PgPoolOptions::new().max_connections(8).connect(url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Db { pool })
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn insert(
        &self,
        id: &str,
        seats: &[Option<Seat>; 2],
        time_control: TimeControl,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO games (id, white_user_id, black_user_id, initial_ms, increment_ms, white_ms, black_ms)
             VALUES ($1, $2, $3, $4, $5, $4, $4)",
            id,
            seats[0].as_ref().map(|s| s.user_id.as_str()),
            seats[1].as_ref().map(|s| s.user_id.as_str()),
            time_control.initial.as_millis() as i64,
            time_control.increment.as_millis() as i64,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_black(&self, id: &str, user_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE games SET black_user_id = $2, updated_at = now() WHERE id = $1",
            id,
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The games a user sits in, newest activity first.
    pub async fn games_of(&self, user_id: &str) -> Result<Vec<GameListing>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"SELECT g.id, g.moves, g.result, g.reason,
                      to_char(g.updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS "updated_at!",
                      g.white_user_id, w.username AS "white_name?",
                      g.black_user_id, b.username AS "black_name?"
               FROM games g
               LEFT JOIN users w ON w.id = g.white_user_id
               LEFT JOIN users b ON b.id = g.black_user_id
               WHERE g.white_user_id = $1 OR g.black_user_id = $1
               ORDER BY g.updated_at DESC
               LIMIT 100"#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                let ended = match (r.result, r.reason) {
                    (Some(res), Some(why)) => Some(GameEnd {
                        result: parse_result(&res).ok_or_else(|| bad_column("result", &res))?,
                        reason: parse_reason(&why).ok_or_else(|| bad_column("reason", &why))?,
                    }),
                    _ => None,
                };
                Ok(GameListing {
                    id: r.id,
                    players: chess_core::protocol::Players {
                        white: r.white_user_id.map(|_| chess_core::protocol::PlayerInfo {
                            username: r.white_name,
                        }),
                        black: r.black_user_id.map(|_| chess_core::protocol::PlayerInfo {
                            username: r.black_name,
                        }),
                    },
                    ended,
                    moves: r.moves.split_whitespace().count() as u32,
                    updated_at: r.updated_at,
                })
            })
            .collect()
    }

    /// Write a room's state. Take the snapshot under the room lock, then call
    /// this without holding it.
    pub async fn save_snapshot(&self, id: &str, snapshot: &Snapshot) -> Result<(), sqlx::Error> {
        let clock_since = snapshot
            .clock_running_for_ms
            .map(|running| unix_ms_now() as i64 - running as i64);
        sqlx::query!(
            "UPDATE games SET moves = $2, white_ms = $3, black_ms = $4, clock_since_unix_ms = $5,
                    draw_offer = $6, result = $7, reason = $8, updated_at = now()
             WHERE id = $1",
            id,
            snapshot.moves.join(" "),
            snapshot.white_ms as i64,
            snapshot.black_ms as i64,
            clock_since,
            snapshot.draw_offer.map(color_str),
            snapshot.ended.map(|e| result_str(e.result)),
            snapshot.ended.map(|e| reason_str(e.reason)),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load(&self, id: &str) -> Result<Option<StoredGame>, sqlx::Error> {
        let Some(row) = sqlx::query!(
            r#"SELECT g.initial_ms, g.increment_ms, g.moves, g.white_ms, g.black_ms,
                      g.clock_since_unix_ms, g.draw_offer, g.result, g.reason,
                      g.white_user_id, w.username AS "white_name?",
                      g.black_user_id, b.username AS "black_name?"
               FROM games g
               LEFT JOIN users w ON w.id = g.white_user_id
               LEFT JOIN users b ON b.id = g.black_user_id
               WHERE g.id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };
        let clock_since = row.clock_since_unix_ms;
        let ended = match (row.result, row.reason) {
            (Some(r), Some(why)) => Some(GameEnd {
                result: parse_result(&r).ok_or_else(|| bad_column("result", &r))?,
                reason: parse_reason(&why).ok_or_else(|| bad_column("reason", &why))?,
            }),
            _ => None,
        };
        let draw_offer = row
            .draw_offer
            .map(|c| parse_color(&c).ok_or_else(|| bad_column("draw_offer", &c)))
            .transpose()?;
        // The clock kept running from `clock_since` until now.
        let (clock_running_for_ms, age) = match clock_since {
            Some(since) => (
                Some((unix_ms_now() as i64 - since).max(0) as u64),
                Duration::ZERO,
            ),
            None => (None, Duration::ZERO),
        };
        let seat = |user_id: Option<String>, username: Option<String>| {
            user_id.map(|user_id| Seat { user_id, username })
        };
        Ok(Some(StoredGame {
            seats: [
                seat(row.white_user_id, row.white_name),
                seat(row.black_user_id, row.black_name),
            ],
            snapshot: Snapshot {
                initial_ms: row.initial_ms as u64,
                increment_ms: row.increment_ms as u64,
                moves: row.moves.split_whitespace().map(str::to_string).collect(),
                white_ms: row.white_ms as u64,
                black_ms: row.black_ms as u64,
                clock_running_for_ms,
                draw_offer,
                ended,
            },
            age,
        }))
    }
}

fn unix_ms_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn bad_column(column: &str, value: &str) -> sqlx::Error {
    sqlx::Error::Decode(format!("games.{column}: unexpected value {value:?}").into())
}

fn color_str(c: Color) -> &'static str {
    if c.is_white() { "white" } else { "black" }
}

fn parse_color(s: &str) -> Option<Color> {
    match s {
        "white" => Some(Color::White),
        "black" => Some(Color::Black),
        _ => None,
    }
}

// The strings match the protocol's serde names, so the database and the
// wire never disagree about what a result is called.
fn result_str(r: GameResult) -> String {
    serde_plain(&r)
}

fn reason_str(r: GameOverReason) -> String {
    serde_plain(&r)
}

fn parse_result(s: &str) -> Option<GameResult> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

fn parse_reason(s: &str) -> Option<GameOverReason> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

fn serde_plain<T: serde::Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        _ => unreachable!("unit enum variants serialize as strings"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_names_match_the_wire_format() {
        assert_eq!(result_str(GameResult::WhiteWins), "white_wins");
        assert_eq!(reason_str(GameOverReason::FiftyMoves), "fifty_moves");
        assert_eq!(parse_result("draw"), Some(GameResult::Draw));
        assert_eq!(parse_reason("timeout"), Some(GameOverReason::Timeout));
        assert_eq!(parse_result("nope"), None);
    }
}
