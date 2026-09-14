//! Public player profiles: `GET /api/players/{username}`.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chess_core::protocol::Category;
use serde::{Deserialize, Serialize};

use crate::{AppState, db::Db};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct PlayerProfile {
    pub username: String,
    /// One entry per category they have played rated games in.
    pub ratings: Vec<CategoryRating>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CategoryRating {
    pub category: Category,
    pub rating: i32,
    pub provisional: bool,
    pub games: u32,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/players/{username}", get(profile))
}

async fn profile(State(state): State<AppState>, Path(username): Path<String>) -> Response {
    let Some(db) = state.auth.as_ref().map(|a| a.db().clone()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    lookup(&db, &username).await
}

async fn lookup(db: &Db, username: &str) -> Response {
    match db.player(username).await {
        Ok(Some(profile)) => Json(profile).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("player profile {username}: {e}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}
