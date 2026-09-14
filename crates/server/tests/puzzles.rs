//! Puzzles over HTTP against the fixture (46 real Lichess puzzles, CC0,
//! rated 610 to 2811): importing, serving, and rating first tries.

mod common;

use chess_core::puzzle::{Puzzle, Verdict};
use common::*;
use server::{
    db::Db,
    puzzles::{AttemptResult, ImportOptions, PuzzleData, import},
};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/puzzles.csv");

async fn with_fixture() -> Option<(Db, String)> {
    let db = db().await?;
    let file = std::fs::File::open(FIXTURE).unwrap();
    let stats = import(&db, file, &ImportOptions::default(), |_| {})
        .await
        .unwrap();
    assert_eq!((stats.read, stats.imported, stats.invalid), (46, 46, 0));
    let base = serve(db.clone()).await;
    Some((db, base))
}

async fn next(base: &str, session: &str) -> Option<PuzzleData> {
    let r = http(base, "GET", "/api/puzzles/next", Some(session), "").await;
    match r.status {
        200 => Some(serde_json::from_str(&r.body).unwrap()),
        404 => None,
        other => panic!("{other}: {}", r.body),
    }
}

async fn attempt(base: &str, session: &str, id: &str, solved: bool) -> AttemptResult {
    let r = http(
        base,
        "POST",
        &format!("/api/puzzles/{id}/attempt"),
        Some(session),
        &format!(r#"{{"solved":{solved}}}"#),
    )
    .await;
    assert_eq!(r.status, 200, "{}", r.body);
    serde_json::from_str(&r.body).unwrap()
}

#[tokio::test]
async fn a_puzzle_near_your_rating_and_only_the_first_try_counts() {
    let Some((_, base)) = with_fixture().await else {
        return;
    };
    assert_eq!(
        http(&base, "GET", "/api/puzzles/next", None, "")
            .await
            .status,
        401
    );
    let me = guest(&base).await;

    // A new solver is 1500?, and gets a puzzle within 100 of that (the
    // fixture has puzzles in every band from 600 to 2800).
    let puzzle = next(&base, &me).await.unwrap();
    assert_eq!(puzzle.your_rating.value, 1500);
    assert!(puzzle.your_rating.provisional);
    assert!((1400..=1600).contains(&puzzle.rating), "{}", puzzle.rating);
    // What's served is solvable with the checker the client runs.
    let moves: Vec<&str> = puzzle.moves.iter().map(String::as_str).collect();
    let p = Puzzle::new(&puzzle.fen, &moves).unwrap();
    let mut played = vec![];
    for (i, m) in moves[1..].iter().enumerate() {
        played.push(*m);
        if i % 2 == 0 {
            let verdict = p.judge(&played).unwrap();
            let last = i + 2 == moves.len();
            assert_eq!(matches!(verdict, Verdict::Solved), last, "{verdict:?}");
        }
    }

    let solved = attempt(&base, &me, &puzzle.id, true).await;
    assert!(solved.counted && solved.diff > 0, "{solved:?}");
    assert_eq!(solved.rating.value, 1500 + solved.diff);
    // Trying again changes nothing.
    let again = attempt(&base, &me, &puzzle.id, false).await;
    assert_eq!(
        again,
        AttemptResult {
            counted: false,
            diff: 0,
            ..solved
        }
    );
    // Failing a new one costs rating.
    let other = next(&base, &me).await.unwrap();
    assert_ne!(other.id, puzzle.id);
    let failed = attempt(&base, &me, &other.id, false).await;
    assert!(failed.counted && failed.diff < 0, "{failed:?}");

    // Unknown puzzles are 404.
    let r = http(
        &base,
        "POST",
        "/api/puzzles/nope/attempt",
        Some(&me),
        r#"{"solved":true}"#,
    )
    .await;
    assert_eq!(r.status, 404);
}

#[tokio::test]
async fn puzzles_are_not_served_twice_and_run_out() {
    let Some((_, base)) = with_fixture().await else {
        return;
    };
    let me = guest(&base).await;
    let mut seen = std::collections::HashSet::new();
    // The shared test database may hold more than the fixture; stop at 46
    // new ones, which proves no repeats, or when they run out.
    while let Some(p) = next(&base, &me).await {
        assert!(seen.insert(p.id.clone()), "served twice: {}", p.id);
        attempt(&base, &me, &p.id, true).await;
        if seen.len() > 46 {
            break;
        }
    }
    assert!(seen.len() >= 46, "{}", seen.len());
}

#[tokio::test]
async fn the_profile_shows_the_puzzle_rating() {
    let Some((_, base)) = with_fixture().await else {
        return;
    };
    let name = format!("solver{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let body = format!(r#"{{"username":"{name}","password":"correct horse"}}"#);
    let me = http(&base, "POST", "/api/auth/signup", None, &body)
        .await
        .cookie
        .unwrap();
    let r = http(&base, "GET", &format!("/api/players/{name}"), None, "").await;
    assert!(r.body.contains(r#""puzzles":null"#), "{}", r.body);
    let p = next(&base, &me).await.unwrap();
    let result = attempt(&base, &me, &p.id, true).await;
    let r = http(&base, "GET", &format!("/api/players/{name}"), None, "").await;
    assert!(
        r.body.contains(&format!(
            r#""puzzles":{{"rating":{},"provisional":true,"attempts":1}}"#,
            result.rating.value
        )),
        "{}",
        r.body
    );
}

#[tokio::test]
async fn the_import_filters_caps_bands_and_checks_moves() {
    let Some(db) = db().await else { return };
    let header = "PuzzleId,FEN,Moves,Rating,RatingDeviation,Popularity,NbPlays,Themes,GameUrl,OpeningTags,DailyDate\n";
    let good = |id: &str, rating: i32| {
        format!(
            "{id},r6k/pp2r2p/4Rp1Q/3p4/8/1N1P2R1/PqP2bPP/7K b - - 0 24,f2g3 e6e7 b2b1 b3c1 b1c1 h6c1,{rating},76,95,10183,crushing,https://lichess.org/x,,\n"
        )
    };
    let tag = uuid::Uuid::new_v4().simple().to_string();
    let csv = [
        header.to_string(),
        good(&format!("{tag}a"), 1510),
        good(&format!("{tag}b"), 1520), // same band as a: over the cap of 1
        good(&format!("{tag}c"), 1610),
        format!("{tag}d,r6k/pp2r2p/4Rp1Q/3p4/8/1N1P2R1/PqP2bPP/7K b - - 0 24,f2g3 e6e7,1500,76,40,10183,x,y,,\n"), // unpopular
        format!("{tag}e,r6k/pp2r2p/4Rp1Q/3p4/8/1N1P2R1/PqP2bPP/7K b - - 0 24,e2e4 e7e5,1700,76,95,10183,x,y,,\n"), // illegal
        format!("{tag}f,not a fen,e2e4 e7e5,1800,76,95,10183,x,y,,\n"),
        "short,line\n".to_string(),
    ]
    .concat();
    let options = ImportOptions {
        per_band: 1,
        ..ImportOptions::default()
    };
    let stats = import(&db, csv.as_bytes(), &options, |_| {}).await.unwrap();
    assert_eq!(stats.read, 7);
    assert_eq!(stats.imported, 2);
    assert_eq!(stats.band_full, 1);
    assert_eq!(stats.filtered, 1);
    assert_eq!(stats.invalid, 3);

    let missing = import(&db, "id,fen\n1,2\n".as_bytes(), &options, |_| {}).await;
    assert!(missing.unwrap_err().to_string().contains("PuzzleId"));
}
