//! The coach over HTTP, against a stand-in for the Anthropic Messages API
//! that records what it was sent.

mod common;

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router, http::HeaderMap, http::StatusCode, response::IntoResponse, routing::post,
};
use common::*;
use server::{Config, coach::CoachConfig};

#[derive(Clone, Default)]
struct Mock {
    calls: Arc<Mutex<Vec<(HeaderMap, serde_json::Value)>>>,
    fail: Arc<Mutex<bool>>,
}

/// A fake `POST /v1/messages`; returns its base URL.
async fn mock_api(mock: Mock) -> String {
    let app = Router::new().route(
        "/v1/messages",
        post(
            move |headers: HeaderMap, Json(body): Json<serde_json::Value>| {
                let mock = mock.clone();
                async move {
                    mock.calls.lock().unwrap().push((headers, body));
                    if *mock.fail.lock().unwrap() {
                        return (StatusCode::INTERNAL_SERVER_ERROR, "overloaded").into_response();
                    }
                    Json(serde_json::json!({
                    "id": "msg_test",
                    "type": "message",
                    "role": "assistant",
                    "model": "claude-opus-5",
                    "content": [{ "type": "text", "text": "  Black fights for d4 with ...c5.  " }],
                    "stop_reason": "end_turn",
                    "usage": { "input_tokens": 10, "output_tokens": 10 }
                }))
                .into_response()
                }
            },
        ),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/v1/messages")
}

async fn coached(per_hour: u32) -> Option<(String, Mock)> {
    let db = db().await?;
    let mock = Mock::default();
    let url = mock_api(mock.clone()).await;
    let base = serve_with(
        db,
        Config {
            coach: Some(CoachConfig {
                api_url: url,
                per_hour,
                ..CoachConfig::anthropic("test-key".into())
            }),
            ..Config::default()
        },
    )
    .await;
    Some((base, mock))
}

async fn account(base: &str) -> String {
    let name = format!("coached{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let body = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    http(base, "POST", "/api/auth/signup", None, &body)
        .await
        .cookie
        .unwrap()
}

/// After 1. e4, Black to move; `first` is the best line's first move.
fn request(first: &str) -> String {
    format!(
        r#"{{"fen":"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1","last_move":"e4",
            "lines":[{{"depth":20,"score":{{"kind":"cp","value":-30}},"pv":["{first}","g1f3"]}}]}}"#
    )
}

#[tokio::test]
async fn explains_from_the_engine_lines_and_caches() {
    let Some((base, mock)) = coached(30).await else {
        return;
    };
    let r = http(&base, "GET", "/api/coach", None, "").await;
    assert_eq!(r.body, r#"{"available":true}"#);

    // Accounts only.
    assert_eq!(
        http(&base, "POST", "/api/coach/explain", None, &request("c7c5"))
            .await
            .status,
        401
    );
    let guest_session = guest(&base).await;
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&guest_session),
        &request("c7c5"),
    )
    .await;
    assert_eq!(r.status, 403, "{}", r.body);

    let me = account(&base).await;
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&me),
        &request("c7c5"),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(r.body, r#"{"text":"Black fights for d4 with ...c5."}"#);

    // What the API was sent: the key, the version, the model, a grounded prompt.
    {
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (headers, body) = &calls[0];
        assert_eq!(headers["x-api-key"], "test-key");
        assert_eq!(headers["anthropic-version"], "2023-06-01");
        assert_eq!(body["model"], "claude-opus-5");
        assert_eq!(body["max_tokens"], 400);
        assert!(body["system"].as_str().unwrap().contains("never invent"));
        let prompt = body["messages"][0]["content"].as_str().unwrap();
        assert_eq!(body["messages"][0]["role"], "user");
        assert!(prompt.contains("Side to move: Black"), "{prompt}");
        assert!(prompt.contains("1. +0.30: 1... c5 2. Nf3"), "{prompt}");
    }

    // The same position and lines: answered from the cache.
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&me),
        &request("c7c5"),
    )
    .await;
    assert_eq!(r.status, 200);
    assert_eq!(mock.calls.lock().unwrap().len(), 1);

    // Nonsense is refused before any API call.
    let bad = r#"{"fen":"nope","last_move":null,"lines":[]}"#;
    let r = http(&base, "POST", "/api/coach/explain", Some(&me), bad).await;
    assert_eq!(r.status, 400, "{}", r.body);
    assert_eq!(mock.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn limits_per_user_and_reports_upstream_failures() {
    let Some((base, mock)) = coached(2).await else {
        return;
    };
    let me = account(&base).await;
    for first in ["c7c5", "e7e5"] {
        let r = http(
            &base,
            "POST",
            "/api/coach/explain",
            Some(&me),
            &request(first),
        )
        .await;
        assert_eq!(r.status, 200, "{}", r.body);
    }
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&me),
        &request("e7e6"),
    )
    .await;
    assert_eq!(r.status, 429, "{}", r.body);
    // Cached answers are still free.
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&me),
        &request("c7c5"),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.body);

    // Someone else, while the API is down: a 502, and nothing cached.
    let other = account(&base).await;
    *mock.fail.lock().unwrap() = true;
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&other),
        &request("d7d5"),
    )
    .await;
    assert_eq!(r.status, 502, "{}", r.body);
    assert!(r.body.contains("unavailable"), "{}", r.body);
    *mock.fail.lock().unwrap() = false;
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&other),
        &request("d7d5"),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.body);
}

#[tokio::test]
async fn off_without_a_key() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    assert_eq!(
        http(&base, "GET", "/api/coach", None, "").await.body,
        r#"{"available":false}"#
    );
    let me = account(&base).await;
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&me),
        &request("c7c5"),
    )
    .await;
    assert_eq!(r.status, 404, "{}", r.body);
}

#[tokio::test]
async fn explains_a_drill_mistake() {
    let Some((base, mock)) = coached(30).await else {
        return;
    };
    let body = r#"{"fen":"8/8/8/3k4/8/8/7Q/4K3 w - - 0 1","played":"h2e5",
        "better":["h2e2","d5d4"],"before":{"kind":"mate","value":8},
        "after":{"kind":"cp","value":0},"drill":"queen-mate"}"#;
    let guest_session = guest(&base).await;
    let r = http(
        &base,
        "POST",
        "/api/coach/mistake",
        Some(&guest_session),
        body,
    )
    .await;
    assert_eq!(r.status, 403, "{}", r.body);

    let me = account(&base).await;
    let r = http(&base, "POST", "/api/coach/mistake", Some(&me), body).await;
    assert_eq!(r.status, 200, "{}", r.body);
    {
        let calls = mock.calls.lock().unwrap();
        let (_, sent) = calls.last().unwrap();
        assert!(sent["system"].as_str().unwrap().contains("made a mistake"));
        let prompt = sent["messages"][0]["content"].as_str().unwrap();
        assert!(prompt.contains("The student played: Qe5+"), "{prompt}");
        assert!(prompt.contains("Drill: King and queen."), "{prompt}");
    }
    // Asked again: from the cache.
    let before = mock.calls.lock().unwrap().len();
    let r = http(&base, "POST", "/api/coach/mistake", Some(&me), body).await;
    assert_eq!(r.status, 200);
    assert_eq!(mock.calls.lock().unwrap().len(), before);

    // An illegal "played" move never reaches the API.
    let bad = body.replace("h2e5", "e1e3");
    let r = http(&base, "POST", "/api/coach/mistake", Some(&me), &bad).await;
    assert_eq!(r.status, 400, "{}", r.body);
    assert_eq!(mock.calls.lock().unwrap().len(), before);
}
