//! Puzzles: importing the Lichess puzzle database, serving each user a
//! puzzle near their puzzle rating (of one theme, if they pick one), the
//! daily puzzle, and rating their first try at each — which is also all that
//! moves their streak.
//!
//! The database (<https://database.lichess.org/#puzzles>, CC0) is a CSV of a
//! few million puzzles. `chess-server import-puzzles` keeps a well-tested
//! subset: popular, often played, with a settled rating, and at most
//! `per_band` puzzles per 100 rating points so every level is covered.

use std::{
    collections::HashMap,
    io::Read,
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chess_core::{protocol::PlayerRating, puzzle::Puzzle};
use serde::{Deserialize, Serialize};
use time::{Date, Month, OffsetDateTime};

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
            // One space between themes, so the SQL can split them.
            themes: text(c_themes)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
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
         SELECT id, fen, moves, rating, deviation, popularity, plays,
                string_to_array(themes, ' ')
         FROM UNNEST($1::text[], $2::text[], $3::text[], $4::int4[], $5::int4[],
                     $6::int4[], $7::int4[], $8::text[])
              AS t(id, fen, moves, rating, deviation, popularity, plays, themes)
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
    /// The solver's streak now.
    pub streak: PuzzleStreak,
    /// Whether the solver has tried this puzzle before, so this try won't
    /// count. Only ever true for the daily puzzle: `next` serves new ones.
    pub tried: bool,
}

/// First tries solved in a row, and the most ever.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PuzzleStreak {
    pub current: i32,
    pub best: i32,
}

/// The puzzle of the day: the same one for everyone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DailyPuzzle {
    /// `YYYY-MM-DD`, in UTC.
    pub date: String,
    pub puzzle: PuzzleData,
}

/// A theme, and how many puzzles have it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PuzzleTheme {
    /// The Lichess theme key, e.g. `fork` or `mateIn2`.
    pub theme: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub count: i64,
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
    /// `false` when this wasn't the first try at the puzzle: nothing changed,
    /// neither the rating nor the streak.
    pub counted: bool,
    /// The streak after this try.
    pub streak: PuzzleStreak,
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

/// A user's streak; nothing yet if they have never tried a puzzle.
pub async fn streak(db: &Db, user_id: &str) -> Result<PuzzleStreak, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT streak, best_streak FROM puzzle_ratings WHERE user_id = $1",
        user_id
    )
    .fetch_optional(db.pool())
    .await?;
    Ok(row
        .map(|r| PuzzleStreak {
            current: r.streak,
            best: r.best_streak,
        })
        .unwrap_or_default())
}

fn shown(rating: Rating) -> PlayerRating {
    PlayerRating {
        value: rating.shown(),
        provisional: rating.provisional(),
    }
}

fn split(moves: &str) -> Vec<String> {
    moves.split_whitespace().map(str::to_string).collect()
}

/// How far from the solver's rating `next_for` looks, nearest first. The last
/// step takes any rating, so a theme that has run dry near the solver's
/// rating still serves something while any puzzle with it is left.
const SPREADS: [i32; 4] = [100, 250, 500, 5000];

/// A random puzzle this user hasn't tried, as near their rating as there is,
/// with `theme` among its themes if one is given. `None` once there is none
/// left at all.
pub async fn next_for(
    db: &Db,
    user_id: &str,
    theme: Option<&str>,
) -> Result<Option<PuzzleData>, sqlx::Error> {
    let (rating, _) = puzzle_rating(db, user_id).await?;
    let center = rating.shown();
    for spread in SPREADS {
        let row = sqlx::query!(
            "SELECT id, fen, moves, rating, themes FROM puzzles p
             WHERE rating BETWEEN $1 AND $2
               AND ($4::text IS NULL OR themes @> ARRAY[$4::text])
               AND NOT EXISTS (SELECT 1 FROM puzzle_attempts a
                               WHERE a.user_id = $3 AND a.puzzle_id = p.id)
             ORDER BY random() LIMIT 1",
            center - spread,
            center + spread,
            user_id,
            theme
        )
        .fetch_optional(db.pool())
        .await?;
        if let Some(p) = row {
            return Ok(Some(PuzzleData {
                id: p.id,
                fen: p.fen,
                moves: split(&p.moves),
                rating: p.rating,
                themes: p.themes,
                your_rating: shown(rating),
                streak: streak(db, user_id).await?,
                tried: false,
            }));
        }
    }
    Ok(None)
}

/// The daily puzzle is drawn from this rating band (or as near it as the
/// pool allows): hard enough to be worth a look, easy enough for most.
pub const DAILY_BAND: (i32, i32) = (1400, 1700);

/// The puzzle for `date` (UTC). The first request for a day picks it —
/// deterministically, from a hash of the date and each puzzle's id, among the
/// puzzles nearest [`DAILY_BAND`] — and stores the pick, so the day keeps its
/// puzzle whatever is imported later. Whether it counts for `user_id` is the
/// ordinary first-try rule. `None` if there are no puzzles.
pub async fn daily_for(
    db: &Db,
    user_id: &str,
    date: Date,
) -> Result<Option<DailyPuzzle>, sqlx::Error> {
    let day = format_date(date);
    sqlx::query!(
        "INSERT INTO daily_puzzles (day, puzzle_id)
         SELECT $1::text::date, id FROM puzzles
         ORDER BY GREATEST($2 - rating, rating - $3, 0), md5($1 || ':' || id), id
         LIMIT 1
         ON CONFLICT (day) DO NOTHING",
        day,
        DAILY_BAND.0,
        DAILY_BAND.1
    )
    .execute(db.pool())
    .await?;
    let Some(p) = sqlx::query!(
        r#"SELECT p.id, p.fen, p.moves, p.rating, p.themes,
                  EXISTS (SELECT 1 FROM puzzle_attempts a
                          WHERE a.user_id = $2 AND a.puzzle_id = p.id) AS "tried!"
           FROM daily_puzzles d JOIN puzzles p ON p.id = d.puzzle_id
           WHERE d.day = $1::text::date"#,
        day,
        user_id
    )
    .fetch_optional(db.pool())
    .await?
    else {
        return Ok(None);
    };
    let (rating, _) = puzzle_rating(db, user_id).await?;
    Ok(Some(DailyPuzzle {
        date: day,
        puzzle: PuzzleData {
            id: p.id,
            fen: p.fen,
            moves: split(&p.moves),
            rating: p.rating,
            themes: p.themes,
            your_rating: shown(rating),
            streak: streak(db, user_id).await?,
            tried: p.tried,
        },
    }))
}

/// Every theme the imported puzzles have, most common first.
pub async fn themes(db: &Db) -> Result<Vec<PuzzleTheme>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT theme AS "theme!", count(*) AS "count!"
           FROM puzzles, unnest(themes) AS theme
           GROUP BY theme ORDER BY count(*) DESC, theme"#
    )
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| PuzzleTheme {
            theme: r.theme,
            count: r.count,
        })
        .collect())
}

/// Record a try. Only the first try at a puzzle counts: it moves the user's
/// puzzle rating as a game against the puzzle's rating would, and their
/// streak (one more if solved, back to nothing if not). A try at a puzzle
/// already tried — however it came to be served again — changes neither.
/// `None` if the puzzle doesn't exist.
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
            streak: streak(db, user_id).await?,
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
    let streak = sqlx::query!(
        "UPDATE puzzle_ratings SET rating = $2, deviation = $3, volatility = $4,
                                   attempts = attempts + 1, last_at = now(),
                                   streak = CASE WHEN $5 THEN streak + 1 ELSE 0 END,
                                   best_streak = GREATEST(best_streak,
                                       CASE WHEN $5 THEN streak + 1 ELSE 0 END)
         WHERE user_id = $1
         RETURNING streak, best_streak",
        user_id,
        after.rating,
        after.deviation,
        after.volatility,
        solved
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(AttemptResult {
        rating: shown(after),
        diff: after.shown() - before.shown(),
        counted: true,
        streak: PuzzleStreak {
            current: streak.streak,
            best: streak.best_streak,
        },
    }))
}

// ----- dates -------------------------------------------------------------

/// `YYYY-MM-DD`.
pub fn format_date(date: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

/// A `YYYY-MM-DD` date that exists.
pub fn parse_date(text: &str) -> Option<Date> {
    let mut parts = text.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let month = Month::try_from(m.parse::<u8>().ok()?).ok()?;
    Date::from_calendar_date(y.parse().ok()?, month, d.parse().ok()?).ok()
}

/// Today in UTC: the daily puzzle changes at midnight UTC for everyone.
pub fn today() -> Date {
    OffsetDateTime::now_utc().date()
}

// ----- HTTP --------------------------------------------------------------

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/puzzles/next", get(next))
        .route("/api/puzzles/themes", get(list_themes))
        .route("/api/puzzles/daily", get(daily_today))
        .route("/api/puzzles/daily/{date}", get(daily_on))
        .route("/api/puzzles/{id}/attempt", post(attempt))
}

fn db_of(state: &AppState) -> Option<Db> {
    state.auth.as_ref().map(|a| a.db().clone())
}

fn unavailable(what: &str, e: sqlx::Error) -> Response {
    tracing::error!("{what}: {e}");
    StatusCode::SERVICE_UNAVAILABLE.into_response()
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

#[derive(Debug, Deserialize)]
struct NextQuery {
    theme: Option<String>,
}

/// A theme key as Lichess writes them: a short run of letters and digits.
fn valid_theme(theme: &str) -> bool {
    (1..=40).contains(&theme.len()) && theme.bytes().all(|b| b.is_ascii_alphanumeric())
}

async fn next(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
    Query(query): Query<NextQuery>,
) -> Response {
    let Some(db) = db_of(&state) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let theme = query.theme.as_deref().filter(|t| !t.is_empty());
    if theme.is_some_and(|t| !valid_theme(t)) {
        return error(StatusCode::BAD_REQUEST, "not a puzzle theme");
    }
    match next_for(&db, &user.id, theme).await {
        Ok(Some(puzzle)) => Json(puzzle).into_response(),
        Ok(None) if theme.is_some() => error(
            StatusCode::NOT_FOUND,
            "no puzzles with this theme left: none have it, or you have tried them all",
        ),
        Ok(None) => error(
            StatusCode::NOT_FOUND,
            "no puzzles to serve: none have been imported, or you have tried them all",
        ),
        Err(e) => unavailable("next puzzle", e),
    }
}

/// How long the theme list is reused: it only changes with an import, and
/// counting every theme of every puzzle reads the whole table.
const THEMES_TTL: Duration = Duration::from_secs(600);

static THEMES_CACHE: Mutex<Option<(Instant, Vec<PuzzleTheme>)>> = Mutex::new(None);

async fn list_themes(State(state): State<AppState>) -> Response {
    let Some(db) = db_of(&state) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let cached = THEMES_CACHE
        .lock()
        .unwrap()
        .as_ref()
        .filter(|(at, list)| at.elapsed() < THEMES_TTL && !list.is_empty())
        .map(|(_, list)| list.clone());
    if let Some(list) = cached {
        return Json(list).into_response();
    }
    match themes(&db).await {
        Ok(list) => {
            *THEMES_CACHE.lock().unwrap() = Some((Instant::now(), list.clone()));
            Json(list).into_response()
        }
        Err(e) => unavailable("puzzle themes", e),
    }
}

async fn daily_today(State(state): State<AppState>, RequireUser(user): RequireUser) -> Response {
    daily(state, user.id, today()).await
}

async fn daily_on(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
    Path(date): Path<String>,
) -> Response {
    let Some(date) = parse_date(&date) else {
        return error(StatusCode::BAD_REQUEST, "not a date: use YYYY-MM-DD");
    };
    // A day's puzzle is picked when the day comes, not before.
    if date > today() {
        return error(StatusCode::NOT_FOUND, "that day hasn't come yet");
    }
    daily(state, user.id, date).await
}

async fn daily(state: AppState, user_id: String, date: Date) -> Response {
    let Some(db) = db_of(&state) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    match daily_for(&db, &user_id, date).await {
        Ok(Some(daily)) => Json(daily).into_response(),
        Ok(None) => error(
            StatusCode::NOT_FOUND,
            "no daily puzzle: none have been imported",
        ),
        Err(e) => unavailable("daily puzzle", e),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip_and_nonsense_is_refused() {
        let d = parse_date("2026-09-22").unwrap();
        assert_eq!(format_date(d), "2026-09-22");
        assert_eq!(format_date(parse_date("2024-02-29").unwrap()), "2024-02-29");
        for bad in [
            "2026-02-30",
            "2025-02-29",
            "2026-13-01",
            "2026-9-22",
            "26-09-22",
            "2026-09-22-1",
            "2026/09/22",
            "",
            "today",
        ] {
            assert_eq!(parse_date(bad), None, "{bad}");
        }
    }

    #[test]
    fn theme_keys() {
        assert!(valid_theme("fork"));
        assert!(valid_theme("mateIn2"));
        assert!(!valid_theme(""));
        assert!(!valid_theme("fork' OR 1=1"));
        assert!(!valid_theme("back rank"));
        assert!(!valid_theme(&"a".repeat(41)));
    }
}
