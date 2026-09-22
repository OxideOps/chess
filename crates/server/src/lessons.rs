//! Lesson progress: which drills an account has finished.
//!
//! Guests keep their progress in the browser (`localStorage`); an account
//! keeps it here, so it follows them between devices. The client merges
//! what the browser holds into the account whenever a signed-in visitor
//! opens the lessons, which is how a guest's progress survives signing up
//! or signing in: the merge is a union, so nothing either side has is lost.
//!
//! Lessons are named by the stable ids in `chess_core::lesson` (never by
//! their place in the list), so reordering or adding drills doesn't move
//! anyone's progress. Ids the server doesn't know (a drill since removed, or
//! junk) are ignored rather than refused, so one stale id can't fail a merge.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, auth::RequireUser, db::Db};

/// The lessons an account has finished, in the order it finished them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct LessonProgress {
    pub completed: Vec<String>,
}

/// Lessons to add to the account: one just finished, or everything a
/// browser remembers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RecordLessons {
    pub completed: Vec<String>,
}

/// The lessons `user_id` has finished, oldest first.
pub async fn completed(db: &Db, user_id: &str) -> Result<LessonProgress, sqlx::Error> {
    let completed = sqlx::query_scalar!(
        "SELECT lesson_id FROM lesson_completions WHERE user_id = $1
         ORDER BY finished_at, lesson_id",
        user_id
    )
    .fetch_all(db.pool())
    .await?;
    Ok(LessonProgress { completed })
}

/// Add finished lessons to `user_id`'s progress and return all of it. A
/// lesson already there keeps its first finish time; unknown ids are
/// dropped.
pub async fn record(
    db: &Db,
    user_id: &str,
    lessons: &[String],
) -> Result<LessonProgress, sqlx::Error> {
    let mut known: Vec<String> = lessons
        .iter()
        .filter(|id| chess_core::lesson::find(id).is_some())
        .cloned()
        .collect();
    known.sort();
    known.dedup();
    if !known.is_empty() {
        sqlx::query!(
            "INSERT INTO lesson_completions (user_id, lesson_id)
             SELECT $1, lesson_id FROM UNNEST($2::text[]) AS l (lesson_id)
             ON CONFLICT DO NOTHING",
            user_id,
            &known
        )
        .execute(db.pool())
        .await?;
    }
    completed(db, user_id).await
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/lessons/completed", get(list).post(add))
}

fn db_of(state: &AppState) -> Option<Db> {
    state.auth.as_ref().map(|a| a.db().clone())
}

fn reply(what: &str, result: Result<LessonProgress, sqlx::Error>) -> Response {
    match result {
        Ok(progress) => Json(progress).into_response(),
        Err(e) => {
            tracing::error!("{what}: {e}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

async fn list(State(state): State<AppState>, RequireUser(user): RequireUser) -> Response {
    let Some(db) = db_of(&state) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    reply("lesson progress", completed(&db, &user.id).await)
}

async fn add(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
    Json(body): Json<RecordLessons>,
) -> Response {
    let Some(db) = db_of(&state) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    reply(
        "record lessons",
        record(&db, &user.id, &body.completed).await,
    )
}
