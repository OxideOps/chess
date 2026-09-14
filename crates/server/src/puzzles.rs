//! Puzzles: importing the Lichess puzzle database, serving each user a
//! puzzle near their puzzle rating, and rating their first try at it.
//!
//! The database (<https://database.lichess.org/#puzzles>, CC0) is a CSV of a
//! few million puzzles. `chess-server import-puzzles` keeps a well-tested
//! subset: popular, often played, with a settled rating, and at most
//! `per_band` puzzles per 100 rating points so every level is covered.

use std::{collections::HashMap, io::Read};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chess_core::{protocol::PlayerRating, puzzle::Puzzle};
use serde::{Deserialize, Serialize};

use crate::{AppState, auth::RequireUser, db::Db, rating::Rating};

const DAY_S: f64 = 86_400.0;
const BATCH: usize = 1000;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub min_popularity: i32,
    pub min_plays: i32,
    pub max_deviation: i32,
    /// Most puzzles kept per 100 rating points.
    pub per_band: usize,
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions {
            min_popularity: 90,
            min_plays: 1000,
            max_deviation: 90,
            per_band: 10_000,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportStats {
    pub read: usize,
    pub imported: usize,
    /// Below the popularity, plays or deviation bar.
    pub filtered: usize,
    /// Their rating band already had `per_band` puzzles.
    pub band_full: usize,
    /// Unparseable, or the moves don't replay.
    pub invalid: usize,
}

struct Row {
    id: String,
    fen: String,
    moves: String,
    rating: i32,
    deviation: i32,
    popularity: i32,
    plays: i32,
    themes: String,
}

/// Load puzzles from a Lichess puzzle CSV. Existing puzzles are updated, so
/// importing again refreshes them. `progress` is called every 100 000 rows.
pub async fn import(
    db: &Db,
    reader: impl Read,
    options: &ImportOptions,
    mut progress: impl FnMut(&ImportStats),
) -> Result<ImportStats, BoxError> {
    let mut csv = csv::ReaderBuilder::new().flexible(true).from_reader(reader);
    let headers = csv.headers()?.clone();
    let column = |name: &str| {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| format!("the file has no {name} column; is it the Lichess puzzle CSV?"))
    };
    let (c_id, c_fen, c_moves) = (column("PuzzleId")?, column("FEN")?, column("Moves")?);
    let (c_rating, c_deviation) = (column("Rating")?, column("RatingDeviation")?);
    let (c_popularity, c_plays, c_themes) =
        (column("Popularity")?, column("NbPlays")?, column("Themes")?);

    let mut stats = ImportStats::default();
    let mut bands: HashMap<i32, usize> = HashMap::new();
    let mut batch = Vec::with_capacity(BATCH);
    for record in csv.records() {
        let record = record?;
        stats.read += 1;
        if stats.read.is_multiple_of(100_000) {
            progress(&stats);
        }
        let text = |i: usize| record.get(i).unwrap_or("").trim();
        let number = |i: usize| text(i).parse::<i32>().ok();
        let (Some(rating), Some(deviation), Some(popularity), Some(plays)) = (
            number(c_rating),
            number(c_deviation),
            number(c_popularity),
            number(c_plays),
        ) else {
            stats.invalid += 1;
            continue;
        };
        if popularity < options.min_popularity
            || plays < options.min_plays
            || deviation > options.max_deviation
        {
            stats.filtered += 1;
            continue;
        }
        let band = bands.entry(rating / 100).or_default();
        if *band >= options.per_band {
            stats.band_full += 1;
            continue;
        }
        let moves: Vec<&str> = text(c_moves).split_whitespace().collect();
        if text(c_id).is_empty() || Puzzle::new(text(c_fen), &moves).is_err() {
            stats.invalid += 1;
            continue;
        }
        *band += 1;
        batch.push(Row {
            id: text(c_id).to_string(),
            fen: text(c_fen).to_string(),
            moves: moves.join(" "),
            rating,
            deviation,
            popularity,
            plays,
            themes: text(c_themes).to_string(),
        });
        if batch.len() == BATCH {
            stats.imported += write(db, &mut batch).await?;
        }
    }
    stats.imported += write(db, &mut batch).await?;
    Ok(stats)
}

async fn write(db: &Db, batch: &mut Vec<Row>) -> Result<usize, sqlx::Error> {
    if batch.is_empty() {
        return Ok(0);
    }
    let rows = std::mem::take(batch);
    let n = rows.len();
    let mut id = Vec::with_capacity(n);
    let mut fen = Vec::with_capacity(n);
    let mut moves = Vec::with_capacity(n);
    let mut rating = Vec::with_capacity(n);
    let mut deviation = Vec::with_capacity(n);
    let mut popularity = Vec::with_capacity(n);
    let mut plays = Vec::with_capacity(n);
    let mut themes = Vec::with_capacity(n);
    for r in rows {
        id.push(r.id);
        fen.push(r.fen);
        moves.push(r.moves);
        rating.push(r.rating);
        deviation.push(r.deviation);
        popularity.push(r.popularity);
        plays.push(r.plays);
        themes.push(r.themes);
    }
    sqlx::query!(
        "INSERT INTO puzzles (id, fen, moves, rating, deviation, popularity, plays, themes)
         SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::int4[], $5::int4[],
                              $6::int4[], $7::int4[], $8::text[])
         ON CONFLICT (id) DO UPDATE SET fen = EXCLUDED.fen, moves = EXCLUDED.moves,
             rating = EXCLUDED.rating, deviation = EXCLUDED.deviation,
             popularity = EXCLUDED.popularity, plays = EXCLUDED.plays, themes = EXCLUDED.themes",
        &id,
        &fen,
        &moves,
        &rating,
        &deviation,
        &popularity,
        &plays,
        &themes,
    )
    .execute(db.pool())
    .await?;
    Ok(n)
}

// ----- serving -----------------------------------------------------------

/// A puzzle to solve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PuzzleData {
    pub id: String,
    /// The position before the opponent's setup move.
    pub fen: String,
    /// UCI: the setup move, then the solution (solver, reply, …, solver).
    pub moves: Vec<String>,
    pub rating: i32,
    pub themes: Vec<String>,
    /// The solver's puzzle rating now.
    pub your_rating: PlayerRating,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PuzzleAttempt {
    pub solved: bool,
}

/// What a try at a puzzle did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct AttemptResult {
    /// The puzzle rating after this try.
    pub rating: PlayerRating,
    pub diff: i32,
    /// `false` when this wasn't the first try at the puzzle: nothing changed.
    pub counted: bool,
}

/// A user's puzzle rating, aged for idle days; the starting rating if new.
pub async fn puzzle_rating(db: &Db, user_id: &str) -> Result<(Rating, u32), sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT rating, deviation, volatility, attempts,
                  EXTRACT(EPOCH FROM now() - last_at)::float8 AS "idle_s!"
           FROM puzzle_ratings WHERE user_id = $1"#,
        user_id
    )
    .fetch_optional(db.pool())
    .await?;
    Ok(match row {
        Some(r) => (
            Rating {
                rating: r.rating,
                deviation: r.deviation,
                volatility: r.volatility,
            }
            .aged(r.idle_s / DAY_S),
            r.attempts as u32,
        ),
        None => (Rating::default(), 0),
    })
}

fn shown(rating: Rating) -> PlayerRating {
    PlayerRating {
        value: rating.shown(),
        provisional: rating.provisional(),
    }
}

/// A random puzzle this user hasn't tried, as near their rating as there is.
pub async fn next_for(db: &Db, user_id: &str) -> Result<Option<PuzzleData>, sqlx::Error> {
    let (rating, _) = puzzle_rating(db, user_id).await?;
    let center = rating.shown();
    for spread in [100, 250, 500, 5000] {
        let row = sqlx::query!(
            "SELECT id, fen, moves, rating, themes FROM puzzles p
             WHERE rating BETWEEN $1 AND $2
               AND NOT EXISTS (SELECT 1 FROM puzzle_attempts a
                               WHERE a.user_id = $3 AND a.puzzle_id = p.id)
             ORDER BY random() LIMIT 1",
            center - spread,
            center + spread,
            user_id
        )
        .fetch_optional(db.pool())
        .await?;
        if let Some(p) = row {
            return Ok(Some(PuzzleData {
                id: p.id,
                fen: p.fen,
                moves: p.moves.split_whitespace().map(str::to_string).collect(),
                rating: p.rating,
                themes: p.themes.split_whitespace().map(str::to_string).collect(),
                your_rating: shown(rating),
            }));
        }
    }
    Ok(None)
}

/// Record a try. Only the first try at a puzzle counts: it moves the user's
/// puzzle rating as a game against the puzzle's rating would. `None` if the
/// puzzle doesn't exist.
pub async fn record_attempt(
    db: &Db,
    user_id: &str,
    puzzle_id: &str,
    solved: bool,
) -> Result<Option<AttemptResult>, sqlx::Error> {
    let mut tx = db.pool().begin().await?;
    let Some(puzzle) = sqlx::query!(
        "SELECT rating, deviation FROM puzzles WHERE id = $1",
        puzzle_id
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(None);
    };
    let first = sqlx::query!(
        "INSERT INTO puzzle_attempts (user_id, puzzle_id, solved) VALUES ($1, $2, $3)
         ON CONFLICT DO NOTHING",
        user_id,
        puzzle_id,
        solved
    )
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if !first {
        tx.commit().await?;
        let (rating, _) = puzzle_rating(db, user_id).await?;
        return Ok(Some(AttemptResult {
            rating: shown(rating),
            diff: 0,
            counted: false,
        }));
    }
    let start = Rating::default();
    sqlx::query!(
        "INSERT INTO puzzle_ratings (user_id, rating, deviation, volatility)
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        user_id,
        start.rating,
        start.deviation,
        start.volatility
    )
    .execute(&mut *tx)
    .await?;
    let row = sqlx::query!(
        r#"SELECT rating, deviation, volatility, attempts,
                  EXTRACT(EPOCH FROM now() - last_at)::float8 AS "idle_s!"
           FROM puzzle_ratings WHERE user_id = $1 FOR UPDATE"#,
        user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let stored = Rating {
        rating: row.rating,
        deviation: row.deviation,
        volatility: row.volatility,
    };
    let before = if row.attempts > 0 {
        stored.aged(row.idle_s / DAY_S)
    } else {
        stored
    };
    let opponent = Rating {
        rating: puzzle.rating as f64,
        deviation: puzzle.deviation as f64,
        volatility: start.volatility,
    };
    let after = before.update(&[(opponent, if solved { 1.0 } else { 0.0 })]);
    sqlx::query!(
        "UPDATE puzzle_ratings SET rating = $2, deviation = $3, volatility = $4,
                                   attempts = attempts + 1, last_at = now()
         WHERE user_id = $1",
        user_id,
        after.rating,
        after.deviation,
        after.volatility
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(AttemptResult {
        rating: shown(after),
        diff: after.shown() - before.shown(),
        counted: true,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/puzzles/next", get(next))
        .route("/api/puzzles/{id}/attempt", post(attempt))
}

fn db_of(state: &AppState) -> Option<Db> {
    state.auth.as_ref().map(|a| a.db().clone())
}

fn unavailable(what: &str, e: sqlx::Error) -> Response {
    tracing::error!("{what}: {e}");
    StatusCode::SERVICE_UNAVAILABLE.into_response()
}

async fn next(State(state): State<AppState>, RequireUser(user): RequireUser) -> Response {
    let Some(db) = db_of(&state) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    match next_for(&db, &user.id).await {
        Ok(Some(puzzle)) => Json(puzzle).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "no puzzles to serve: none have been imported, or you have tried them all" })),
        )
            .into_response(),
        Err(e) => unavailable("next puzzle", e),
    }
}

async fn attempt(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Json(body): Json<PuzzleAttempt>,
) -> Response {
    let Some(db) = db_of(&state) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    match record_attempt(&db, &user.id, &id, body.solved).await {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => unavailable("puzzle attempt", e),
    }
}
