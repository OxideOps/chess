//! The coach over HTTP, against a stand-in for the Anthropic Messages API
//! that records what it was sent.

mod common;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router, http::HeaderMap, http::StatusCode, response::IntoResponse, routing::post,
};
use common::*;
use server::{Config, coach::CoachConfig};

#[derive(Clone, Default)]
struct Mock {
    calls: Arc<Mutex<Vec<(HeaderMap, serde_json::Value)>>>,
    fail: Arc<Mutex<bool>>,
    /// A `stop_reason` other than `end_turn`, e.g. a cut-off answer.
    stop: Arc<Mutex<Option<&'static str>>>,
    /// The next answers, in order (`None`: a 500); then the usual one.
    script: Arc<Mutex<VecDeque<Option<&'static str>>>>,
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
                    let text = match mock.script.lock().unwrap().pop_front() {
                        Some(Some(text)) => text,
                        Some(None) => {
                            return (StatusCode::INTERNAL_SERVER_ERROR, "overloaded")
                                .into_response();
                        }
                        None => "  Black fights for d4 with ...c5.  ",
                    };
                    let stop = mock.stop.lock().unwrap().unwrap_or("end_turn");
                    Json(serde_json::json!({
                        "id": "msg_test",
                        "type": "message",
                        "role": "assistant",
                        "model": "claude-opus-5",
                        "content": [
                            { "type": "thinking", "thinking": "", "signature": "sig" },
                            { "type": "text", "text": text }
                        ],
                        "stop_reason": stop,
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
    // The text, the same text with the move from the lines marked, and a
    // thread for follow-ups.
    let mut body: serde_json::Value = serde_json::from_str(&r.body).unwrap();
    let thread = body.as_object_mut().unwrap().remove("thread").unwrap();
    assert_eq!(thread.as_str().unwrap().len(), 32);
    assert_eq!(
        body,
        serde_json::json!({
            "text": "Black fights for d4 with ...c5.",
            "parts": [
                { "kind": "text", "text": "Black fights for d4 with ..." },
                { "kind": "move", "text": "c5", "path": ["c7c5"] },
                { "kind": "text", "text": "." },
            ]
        })
    );

    // What the API was sent: the key, the version, the model, a grounded prompt.
    {
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (headers, body) = &calls[0];
        assert_eq!(headers["x-api-key"], "test-key");
        assert_eq!(headers["anthropic-version"], "2023-06-01");
        assert_eq!(headers["anthropic-beta"], "server-side-fallback-2026-07-01");
        assert_eq!(body["model"], "claude-opus-5");
        assert_eq!(body["max_tokens"], 4000);
        assert_eq!(body["fallbacks"], "default");
        let system = body["system"].as_str().unwrap();
        assert!(system.contains("never invent"), "{system}");
        assert!(system.contains("no Markdown"), "{system}");
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
        "after":{"kind":"cp","value":0},"drill":"queen-mate","reply":["d5e5"]}"#;
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
        assert!(prompt.contains("The student played Qe5+:"), "{prompt}");
        assert!(
            prompt.contains("- 1... Kxe5: the black king on d5 takes the white queen on e5."),
            "{prompt}"
        );
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

#[tokio::test]
async fn a_cut_off_or_declined_answer_is_an_error() {
    let Some((base, mock)) = coached(30).await else {
        return;
    };
    let me = account(&base).await;
    for stop in ["max_tokens", "refusal"] {
        *mock.stop.lock().unwrap() = Some(stop);
        let r = http(
            &base,
            "POST",
            "/api/coach/explain",
            Some(&me),
            &request("c7c5"),
        )
        .await;
        assert_eq!(r.status, 502, "{stop}: {}", r.body);
    }
    // Nothing was cached: asked again, the API answers in full.
    *mock.stop.lock().unwrap() = None;
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&me),
        &request("c7c5"),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(mock.calls.lock().unwrap().len(), 3);
}

/// The `text` of an explanation.
fn text_of(body: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(body).unwrap();
    v["text"].as_str().unwrap().to_string()
}

/// The messages of the `n`th call.
fn messages(mock: &Mock, n: usize) -> Vec<serde_json::Value> {
    mock.calls.lock().unwrap()[n].1["messages"]
        .as_array()
        .unwrap()
        .clone()
}

#[tokio::test]
async fn a_flagged_answer_is_corrected_once() {
    let Some((base, mock)) = coached(30).await else {
        return;
    };
    let me = account(&base).await;
    let explain = |first: &'static str| {
        let base = base.clone();
        let me = me.clone();
        async move {
            http(
                &base,
                "POST",
                "/api/coach/explain",
                Some(&me),
                &request(first),
            )
            .await
        }
    };

    // Qe7 isn't in the lines: flagged, and the rewrite (clean) is served.
    mock.script.lock().unwrap().extend([
        Some("Black should answer with Qe7."),
        Some("Black answers with ...c5, fighting for d4."),
    ]);
    let r = explain("c7c5").await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        text_of(&r.body),
        "Black answers with ...c5, fighting for d4."
    );
    assert_eq!(mock.calls.lock().unwrap().len(), 2);
    // The correction turn: the question, the first answer echoed exactly as
    // it came (thinking block and all), then what was wrong with it.
    let turns = messages(&mock, 1);
    assert_eq!(turns.len(), 3);
    assert_eq!(turns[0], messages(&mock, 0)[0]);
    assert_eq!(turns[1]["role"], "assistant");
    assert_eq!(turns[1]["content"][0]["type"], "thinking");
    assert_eq!(
        turns[1]["content"][1]["text"],
        "Black should answer with Qe7."
    );
    assert_eq!(turns[2]["role"], "user");
    let asked = turns[2]["content"].as_str().unwrap();
    assert!(
        asked.contains("- It mentions Qe7, which isn't in the lines it was given."),
        "{asked}"
    );
    // The served rewrite is what's cached.
    let r = explain("c7c5").await;
    assert_eq!(
        text_of(&r.body),
        "Black answers with ...c5, fighting for d4."
    );
    assert_eq!(mock.calls.lock().unwrap().len(), 2);

    // A rewrite that's no better: the first answer stands. Only one retry.
    mock.script.lock().unwrap().extend([
        Some("Black plays Qe7 here."),
        Some("Black still plays Qe7."),
    ]);
    let r = explain("e7e5").await;
    assert_eq!(text_of(&r.body), "Black plays Qe7 here.");
    assert_eq!(mock.calls.lock().unwrap().len(), 4);

    // The correction call fails: the first answer still stands.
    mock.script
        .lock()
        .unwrap()
        .extend([Some("Black plays Qe7 now."), None]);
    let r = explain("d7d5").await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(text_of(&r.body), "Black plays Qe7 now.");
    assert_eq!(mock.calls.lock().unwrap().len(), 6);

    // A clean answer is never re-asked.
    mock.script
        .lock()
        .unwrap()
        .push_back(Some("Black develops the knight on f6."));
    let r = explain("g8f6").await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(mock.calls.lock().unwrap().len(), 7);
}

/// POST a follow-up; the status and the reply.
async fn follow_up(base: &str, who: &str, body: serde_json::Value) -> (u16, serde_json::Value) {
    let r = http(
        base,
        "POST",
        "/api/coach/followup",
        Some(who),
        &body.to_string(),
    )
    .await;
    let reply = serde_json::from_str(&r.body).unwrap_or(serde_json::Value::Null);
    (r.status, reply)
}

#[tokio::test]
async fn follow_up_questions_carry_the_conversation_on() {
    let Some((base, mock)) = coached(30).await else {
        return;
    };
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
    let explained: serde_json::Value = serde_json::from_str(&r.body).unwrap();
    let thread = explained["thread"].as_str().unwrap().to_string();

    // A question: the conversation so far, the model's turn exactly as it
    // came, then the question; cached for the next one.
    let (status, reply) = follow_up(
        &base,
        &me,
        serde_json::json!({ "thread": thread, "question": "Why is c5 good?" }),
    )
    .await;
    assert_eq!(status, 200, "{reply}");
    assert_eq!(reply["kind"], "answer");
    assert_eq!(reply["left"], 4);
    assert_eq!(reply["answer"]["thread"], thread.as_str());
    assert_eq!(reply["answer"]["text"], "Black fights for d4 with ...c5.");
    let turns = messages(&mock, 1);
    assert_eq!(turns.len(), 3);
    assert_eq!(turns[0], messages(&mock, 0)[0]);
    assert_eq!(turns[1]["content"][0]["type"], "thinking");
    let asked = turns[2]["content"].as_str().unwrap();
    assert!(
        asked.contains("<question>\nWhy is c5 good?\n</question>"),
        "{asked}"
    );
    assert!(asked.contains("say you would need the engine"), "{asked}");
    assert_eq!(
        mock.calls.lock().unwrap()[1].1["cache_control"]["type"],
        "ephemeral"
    );

    // A move the lines don't start with: Stockfish first, no model call.
    let calls = mock.calls.lock().unwrap().len();
    let (status, reply) = follow_up(
        &base,
        &me,
        serde_json::json!({ "thread": thread, "question": "Why not Nf6?" }),
    )
    .await;
    assert_eq!(status, 200, "{reply}");
    assert_eq!(
        reply,
        serde_json::json!({
            "kind": "probe",
            "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
            "uci": "g8f6",
            "san": "Nf6",
        })
    );
    assert_eq!(mock.calls.lock().unwrap().len(), calls);
    // With Stockfish's line, it goes to the model, spelled out; the answer
    // can name the new moves.
    mock.script
        .lock()
        .unwrap()
        .push_back(Some("After 1... Nf6 2. e5 the knight is chased."));
    let (status, reply) = follow_up(
        &base,
        &me,
        serde_json::json!({
            "thread": thread,
            "question": "Why not Nf6?",
            "probe": {
                "uci": "g8f6",
                "line": { "depth": 18, "score": { "kind": "cp", "value": 40 }, "pv": ["e4e5", "f6d5"] }
            }
        }),
    )
    .await;
    assert_eq!(status, 200, "{reply}");
    assert_eq!(reply["left"], 3);
    let turns = messages(&mock, calls);
    // The thread grew: question 1 and its answer come before this one.
    assert_eq!(turns.len(), 5);
    let asked = turns[4]["content"].as_str().unwrap();
    for fact in [
        "- 1... Nf6: the black knight on g8 moves to f6, and now attacks the white pawn on e4.",
        "- 2. e5: the white pawn on e4 moves to e5, and now attacks the black knight on f6.",
        "from White's point of view, at depth 18",
    ] {
        assert!(asked.contains(fact), "{fact}\n\n{asked}");
    }
    let parts = reply["answer"]["parts"].as_array().unwrap();
    let marked: Vec<&str> = parts
        .iter()
        .filter(|p| p["kind"] == "move")
        .map(|p| p["text"].as_str().unwrap())
        .collect();
    assert_eq!(marked, ["Nf6", "e5"]);

    // Someone else's thread, a made-up one, a guest, and a bad question.
    let other = account(&base).await;
    let (status, _) = follow_up(
        &base,
        &other,
        serde_json::json!({ "thread": thread, "question": "Why?" }),
    )
    .await;
    assert_eq!(status, 404);
    let (status, _) = follow_up(
        &base,
        &me,
        serde_json::json!({ "thread": "nope", "question": "Why?" }),
    )
    .await;
    assert_eq!(status, 404);
    let guest_session = guest(&base).await;
    let (status, _) = follow_up(
        &base,
        &guest_session,
        serde_json::json!({ "thread": thread, "question": "Why?" }),
    )
    .await;
    assert_eq!(status, 403);
    for question in ["  ", &"why ".repeat(100)] {
        let (status, _) = follow_up(
            &base,
            &me,
            serde_json::json!({ "thread": thread, "question": question }),
        )
        .await;
        assert_eq!(status, 400);
    }
    // An illegal probe is refused.
    let (status, _) = follow_up(
        &base,
        &me,
        serde_json::json!({
            "thread": thread, "question": "Why not Nf6?",
            "probe": { "uci": "e2e4", "line": { "depth": 1, "score": { "kind": "cp", "value": 0 }, "pv": [] } }
        }),
    )
    .await;
    assert_eq!(status, 400);

    // Five questions per answer.
    for left in [2, 1, 0] {
        let (status, reply) = follow_up(
            &base,
            &me,
            serde_json::json!({ "thread": thread, "question": "And then?" }),
        )
        .await;
        assert_eq!(status, 200, "{reply}");
        assert_eq!(reply["left"], left);
    }
    let (status, reply) = follow_up(
        &base,
        &me,
        serde_json::json!({ "thread": thread, "question": "And then?" }),
    )
    .await;
    assert_eq!(status, 429, "{reply}");
}

#[tokio::test]
async fn follow_ups_count_against_the_hourly_limit() {
    let Some((base, _mock)) = coached(2).await else {
        return;
    };
    let me = account(&base).await;
    let r = http(
        &base,
        "POST",
        "/api/coach/explain",
        Some(&me),
        &request("c7c5"),
    )
    .await;
    let thread = serde_json::from_str::<serde_json::Value>(&r.body).unwrap()["thread"]
        .as_str()
        .unwrap()
        .to_string();
    let ask = || {
        follow_up(
            &base,
            &me,
            serde_json::json!({ "thread": thread, "question": "Why?" }),
        )
    };
    assert_eq!(ask().await.0, 200);
    assert_eq!(ask().await.0, 429);
}
