//! Lesson progress over HTTP: an account's finished lessons follow it to
//! another device, and what a guest's browser remembers merges into the
//! account when they sign up or sign in.

mod common;

use common::*;
use server::lessons::LessonProgress;

async fn post(base: &str, session: &str, ids: &[&str]) -> Vec<String> {
    let body = serde_json::json!({ "completed": ids }).to_string();
    let r = http(base, "POST", "/api/lessons/completed", Some(session), &body).await;
    assert_eq!(r.status, 200, "{}", r.body);
    serde_json::from_str::<LessonProgress>(&r.body)
        .unwrap()
        .completed
}

async fn list(base: &str, session: &str) -> Vec<String> {
    let r = http(base, "GET", "/api/lessons/completed", Some(session), "").await;
    assert_eq!(r.status, 200, "{}", r.body);
    serde_json::from_str::<LessonProgress>(&r.body)
        .unwrap()
        .completed
}

async fn login(base: &str, name: &str) -> String {
    let body = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let r = http(base, "POST", "/api/auth/login", None, &body).await;
    assert_eq!(r.status, 200, "{}", r.body);
    r.cookie.expect("login gets a session cookie")
}

#[tokio::test]
async fn progress_belongs_to_the_account_and_follows_it() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    for method in ["GET", "POST"] {
        let r = http(
            &base,
            method,
            "/api/lessons/completed",
            None,
            r#"{"completed":[]}"#,
        )
        .await;
        assert_eq!(r.status, 401, "{method}");
    }

    let name = unique("lsn_");
    let laptop = signup(&base, &name).await;
    assert!(list(&base, &laptop).await.is_empty());
    assert_eq!(
        post(&base, &laptop, &["back-rank-mate"]).await,
        ["back-rank-mate"]
    );
    // Finishing it again changes nothing.
    assert_eq!(
        post(&base, &laptop, &["back-rank-mate"]).await,
        ["back-rank-mate"]
    );

    // Another device, another session: the same progress.
    let phone = login(&base, &name).await;
    assert_eq!(list(&base, &phone).await, ["back-rank-mate"]);
    assert_eq!(
        post(&base, &phone, &["two-rooks"]).await,
        ["back-rank-mate", "two-rooks"]
    );
    assert_eq!(list(&base, &laptop).await, ["back-rank-mate", "two-rooks"]);

    // Someone else's account sees none of it.
    let other = signup(&base, &unique("lsn_")).await;
    assert!(list(&base, &other).await.is_empty());
}

#[tokio::test]
async fn a_guests_browser_progress_merges_into_the_account() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;

    // A guest finishes lessons; the browser remembers them, the server doesn't.
    let session = guest(&base).await;
    assert!(list(&base, &session).await.is_empty());

    // They sign up: the same session, upgraded in place.
    let name = unique("lsn_");
    let body = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let r = http(&base, "POST", "/api/auth/signup", Some(&session), &body).await;
    assert_eq!(r.status, 201, "{}", r.body);

    // The client sends what the browser held. Duplicates and ids the server
    // doesn't know (a removed drill, junk) are dropped, not refused.
    assert_eq!(
        post(
            &base,
            &session,
            &["two-rooks", "back-rank-mate", "two-rooks", "no-such-lesson"]
        )
        .await,
        ["back-rank-mate", "two-rooks"]
    );

    // Later, on another browser that has progress of its own, they sign in:
    // the merge is a union, so neither side loses anything.
    let elsewhere = login(&base, &name).await;
    assert_eq!(
        post(&base, &elsewhere, &["queen-mate", "back-rank-mate"]).await,
        ["back-rank-mate", "two-rooks", "queen-mate"]
    );
    // Nothing held locally: the merge is just a read.
    assert_eq!(
        post(&base, &session, &[]).await,
        ["back-rank-mate", "two-rooks", "queen-mate"]
    );
}
