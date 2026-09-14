//! Postgres persistence for games.
//!
//! The in-memory [`Room`](crate::room::Room) stays the source of truth while
//! the server runs; every change is written through as one row per game,
//! and a game that isn't in memory is loaded from here on first access.
//! Running without a database is allowed (nothing survives a restart then).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chess_core::{
    Color,
    protocol::{Category, GameEnd, GameOverReason, GameResult, PlayerRating, RatingDiffs},
};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    games::{GameListing, Seat},
    players::{CategoryRating, PlayerProfile, PuzzleRatingSummary},
    rating::{Rating, rate_game},
    room::{Snapshot, TimeControl},
};

/// A rating period: deviations grow back by one period per idle day.
const DAY_S: f64 = 86_400.0;

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
    pub rated: bool,
    pub rating_diffs: Option<RatingDiffs>,
}

fn player_rating(value: Option<i32>, provisional: Option<bool>) -> Option<PlayerRating> {
    value.map(|value| PlayerRating {
        value,
        provisional: provisional.unwrap_or(true),
    })
}

fn rating_diffs(white: Option<i32>, black: Option<i32>) -> Option<RatingDiffs> {
    Some(RatingDiffs {
        white: white?,
        black: black?,
    })
}

impl Db {
    /// Connect and apply migrations.
    pub async fn connect(url: &str) -> Result<Db, sqlx::Error> {
        let pool = PgPoolOptions::new().max_connections(8).connect(url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Db { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn insert(
        &self,
        id: &str,
        seats: &[Option<Seat>; 2],
        time_control: TimeControl,
        rated: bool,
    ) -> Result<(), sqlx::Error> {
        let rating = |i: usize| seats[i].as_ref().and_then(|s| s.rating);
        sqlx::query!(
            "INSERT INTO games (id, white_user_id, black_user_id, initial_ms, increment_ms, white_ms, black_ms,
                                rated, white_rating, white_provisional, black_rating, black_provisional)
             VALUES ($1, $2, $3, $4, $5, $4, $4, $6, $7, $8, $9, $10)",
            id,
            seats[0].as_ref().map(|s| s.user_id.as_str()),
            seats[1].as_ref().map(|s| s.user_id.as_str()),
            time_control.initial.as_millis() as i64,
            time_control.increment.as_millis() as i64,
            rated,
            rating(0).map(|r| r.value),
            rating(0).map(|r| r.provisional),
            rating(1).map(|r| r.value),
            rating(1).map(|r| r.provisional),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_black(&self, id: &str, seat: &Seat) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE games SET black_user_id = $2, black_rating = $3, black_provisional = $4,
                              updated_at = now()
             WHERE id = $1",
            id,
            seat.user_id,
            seat.rating.map(|r| r.value),
            seat.rating.map(|r| r.provisional),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// A user's rating in `category`, with the deviation grown for the days
    /// since their last game; the starting rating if they have none yet.
    pub async fn rating(&self, user_id: &str, category: Category) -> Result<Rating, sqlx::Error> {
        let row = sqlx::query!(
            r#"SELECT rating, deviation, volatility,
                      EXTRACT(EPOCH FROM now() - last_game_at)::float8 AS "idle_s!"
               FROM ratings WHERE user_id = $1 AND category = $2"#,
            user_id,
            category.as_str()
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row
            .map(|r| {
                Rating {
                    rating: r.rating,
                    deviation: r.deviation,
                    volatility: r.volatility,
                }
                .aged(r.idle_s / DAY_S)
            })
            .unwrap_or_default())
    }

    /// Update both players' ratings for a finished rated game, once: the
    /// game row's `rating_applied` flag is set in the same transaction, so a
    /// second call (another end signal, a restart) finds nothing to do.
    /// `None` when there was nothing to apply (casual, aborted, unfinished,
    /// or already applied).
    pub async fn apply_ratings(&self, id: &str) -> Result<Option<RatingDiffs>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let Some(game) = sqlx::query!(
            r#"UPDATE games SET rating_applied = true
               WHERE id = $1 AND rated AND NOT rating_applied
                 AND result IN ('white_wins', 'black_wins', 'draw')
                 AND white_user_id IS NOT NULL AND black_user_id IS NOT NULL
               RETURNING white_user_id AS "white!", black_user_id AS "black!",
                         result AS "result!", initial_ms, increment_ms"#,
            id
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            tx.rollback().await?;
            return Ok(None);
        };
        let white_score = match parse_result(&game.result) {
            Some(GameResult::WhiteWins) => 1.0,
            Some(GameResult::BlackWins) => 0.0,
            Some(GameResult::Draw) => 0.5,
            _ => return Err(bad_column("result", &game.result)),
        };
        let category = Category::of(game.initial_ms as u64, game.increment_ms as u64);

        // Create missing rows, then lock both, always in the same order, so
        // two games between the same players ending at once can't deadlock.
        let mut users = [game.white.clone(), game.black.clone()];
        users.sort();
        let start = Rating::default();
        for user in &users {
            sqlx::query!(
                "INSERT INTO ratings (user_id, category, rating, deviation, volatility)
                 VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
                user,
                category.as_str(),
                start.rating,
                start.deviation,
                start.volatility
            )
            .execute(&mut *tx)
            .await?;
        }
        let rows = sqlx::query!(
            r#"SELECT user_id, rating, deviation, volatility, games,
                      EXTRACT(EPOCH FROM now() - last_game_at)::float8 AS "idle_s!"
               FROM ratings WHERE category = $1 AND user_id = ANY($2)
               ORDER BY user_id FOR UPDATE"#,
            category.as_str(),
            &users[..]
        )
        .fetch_all(&mut *tx)
        .await?;
        let before = |user: &str| {
            rows.iter()
                .find(|r| r.user_id == user)
                .map(|r| {
                    let rating = Rating {
                        rating: r.rating,
                        deviation: r.deviation,
                        volatility: r.volatility,
                    };
                    if r.games > 0 {
                        rating.aged(r.idle_s / DAY_S)
                    } else {
                        rating
                    }
                })
                .ok_or(sqlx::Error::RowNotFound)
        };
        let (white_before, black_before) = (before(&game.white)?, before(&game.black)?);
        let (white_after, black_after) = rate_game(white_before, black_before, white_score);
        for (user, after) in [(&game.white, white_after), (&game.black, black_after)] {
            sqlx::query!(
                "UPDATE ratings SET rating = $3, deviation = $4, volatility = $5,
                                    games = games + 1, last_game_at = now()
                 WHERE user_id = $1 AND category = $2",
                user,
                category.as_str(),
                after.rating,
                after.deviation,
                after.volatility
            )
            .execute(&mut *tx)
            .await?;
        }
        let diffs = RatingDiffs {
            white: white_after.shown() - white_before.shown(),
            black: black_after.shown() - black_before.shown(),
        };
        sqlx::query!(
            "UPDATE games SET white_rating_diff = $2, black_rating_diff = $3 WHERE id = $1",
            id,
            diffs.white,
            diffs.black
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(diffs))
    }

    /// A registered player's public profile: their ratings per category
    /// played. `None` for unknown names (and guests, who have none).
    pub async fn player(&self, username: &str) -> Result<Option<PlayerProfile>, sqlx::Error> {
        let Some(user) = sqlx::query!(
            r#"SELECT id, username AS "username!" FROM users
               WHERE lower(username) = lower($1) AND NOT is_guest"#,
            username
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };
        let rows = sqlx::query!(
            r#"SELECT category, rating, deviation, volatility, games,
                      EXTRACT(EPOCH FROM now() - last_game_at)::float8 AS "idle_s!"
               FROM ratings WHERE user_id = $1 AND games > 0"#,
            user.id
        )
        .fetch_all(&self.pool)
        .await?;
        let mut ratings = Vec::new();
        for category in Category::ALL {
            if let Some(r) = rows.iter().find(|r| r.category == category.as_str()) {
                let rating = Rating {
                    rating: r.rating,
                    deviation: r.deviation,
                    volatility: r.volatility,
                }
                .aged(r.idle_s / DAY_S);
                ratings.push(CategoryRating {
                    category,
                    rating: rating.shown(),
                    provisional: rating.provisional(),
                    games: r.games as u32,
                });
            }
        }
        let (puzzle, attempts) = crate::puzzles::puzzle_rating(self, &user.id).await?;
        Ok(Some(PlayerProfile {
            username: user.username,
            ratings,
            puzzles: (attempts > 0).then(|| PuzzleRatingSummary {
                rating: puzzle.shown(),
                provisional: puzzle.provisional(),
                attempts,
            }),
        }))
    }

    /// The games a user sits in, newest activity first.
    pub async fn games_of(&self, user_id: &str) -> Result<Vec<GameListing>, sqlx::Error> {
        let rows = sqlx::query!(
            // Usernames come from subselects rather than LEFT JOINs: see `load`.
            r#"SELECT g.id, g.moves, g.result, g.reason,
                      to_char(g.updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS "updated_at!",
                      g.white_user_id,
                      (SELECT username FROM users WHERE id = g.white_user_id) AS "white_name?",
                      g.black_user_id,
                      (SELECT username FROM users WHERE id = g.black_user_id) AS "black_name?",
                      g.rated, g.white_rating, g.white_provisional, g.black_rating,
                      g.black_provisional, g.white_rating_diff, g.black_rating_diff
               FROM games g
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
                let your_color = if r.white_user_id.as_deref() == Some(user_id) {
                    Color::White
                } else {
                    Color::Black
                };
                Ok(GameListing {
                    id: r.id,
                    players: chess_core::protocol::Players {
                        white: r.white_user_id.map(|_| chess_core::protocol::PlayerInfo {
                            username: r.white_name,
                            rating: player_rating(r.white_rating, r.white_provisional),
                        }),
                        black: r.black_user_id.map(|_| chess_core::protocol::PlayerInfo {
                            username: r.black_name,
                            rating: player_rating(r.black_rating, r.black_provisional),
                        }),
                    },
                    your_color,
                    rated: r.rated,
                    rating_diffs: rating_diffs(r.white_rating_diff, r.black_rating_diff),
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
            // The usernames are scalar subselects, not LEFT JOINs, on purpose:
            // sqlx infers nullability from the query plan and writes it to the
            // offline cache, and with two outer joins the planner's join order
            // (and so the inferred nullability of the `games` columns) changed
            // with the table size, making `cargo sqlx prepare --check` fail on
            // a database of a different size. A plain scan is stable.
            r#"SELECT g.initial_ms, g.increment_ms, g.moves, g.white_ms, g.black_ms,
                      g.clock_since_unix_ms, g.draw_offer, g.result, g.reason,
                      g.white_user_id,
                      (SELECT username FROM users WHERE id = g.white_user_id) AS "white_name?",
                      g.black_user_id,
                      (SELECT username FROM users WHERE id = g.black_user_id) AS "black_name?",
                      g.rated, g.white_rating, g.white_provisional, g.black_rating,
                      g.black_provisional, g.white_rating_diff, g.black_rating_diff
               FROM games g
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
        let seat = |user_id: Option<String>, username: Option<String>, rating| {
            user_id.map(|user_id| Seat {
                user_id,
                username,
                rating,
            })
        };
        Ok(Some(StoredGame {
            seats: [
                seat(
                    row.white_user_id,
                    row.white_name,
                    player_rating(row.white_rating, row.white_provisional),
                ),
                seat(
                    row.black_user_id,
                    row.black_name,
                    player_rating(row.black_rating, row.black_provisional),
                ),
            ],
            rated: row.rated,
            rating_diffs: rating_diffs(row.white_rating_diff, row.black_rating_diff),
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
