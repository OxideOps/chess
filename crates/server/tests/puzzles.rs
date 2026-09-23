//! Puzzles over HTTP against the fixture (46 real Lichess puzzles, CC0,
//! rated 610 to 2811): importing, serving (by theme too), the daily puzzle,
//! streaks, and rating first tries — and only first tries.

mod common;

use chess_core::puzzle::{Puzzle, Verdict};
use common::*;
use server::{
    db::Db,
    puzzles::{
        AttemptResult, DAILY_BAND, DailyPuzzle, ImportOptions, PuzzleData, PuzzleStreak,
        PuzzleTheme, format_date, import, today,
    },
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

/// The fixture's puzzles with `theme`: (id, rating).
fn fixture_with(theme: &str) -> Vec<(String, i32)> {
    let mut csv = csv::Reader::from_path(FIXTURE).unwrap();
    csv.records()
        .map(Result::unwrap)
        .filter(|r| r[7].split_whitespace().any(|t| t == theme))
        .map(|r| (r[0].to_string(), r[3].parse().unwrap()))
        .collect()
}

async fn next_themed(base: &str, session: &str, theme: &str) -> Option<PuzzleData> {
    let r = http(
        base,
        "GET",
        &format!("/api/puzzles/next?theme={theme}"),
        Some(session),
        "",
    )
    .await;
    match r.status {
        200 => Some(serde_json::from_str(&r.body).unwrap()),
        404 => {
            assert!(r.body.contains("theme"), "{}", r.body);
            None
        }
        other => panic!("{other}: {}", r.body),
    }
}

async fn daily(base: &str, session: &str, path: &str) -> DailyPuzzle {
    let r = http(base, "GET", path, Some(session), "").await;
    assert_eq!(r.status, 200, "{}", r.body);
    serde_json::from_str(&r.body).unwrap()
}

#[tokio::test]
async fn themes_are_listed_and_a_theme_widens_before_it_runs_out() {
    let Some((_, base)) = with_fixture().await else {
        return;
    };
    let r = http(&base, "GET", "/api/puzzles/themes", None, "").await;
    assert_eq!(r.status, 200, "{}", r.body);
    let themes: Vec<PuzzleTheme> = serde_json::from_str(&r.body).unwrap();
    assert!(themes.windows(2).all(|w| w[0].count >= w[1].count));
    let count = |name: &str| themes.iter().find(|t| t.theme == name).map(|t| t.count);
    assert!(count("fork").unwrap() >= 7);
    assert!(count("anastasiaMate").unwrap() >= 1);
    assert_eq!(count(""), None);

    // The fixture's seven forks are rated 701 to 1495. A new solver (1500)
    // gets the one within 100 first, then the rest as the range widens,
    // every one a fork, none twice, and then a 404 that names the theme.
    let forks = fixture_with("fork");
    assert_eq!(forks.len(), 7);
    let me = guest(&base).await;
    let first = next_themed(&base, &me, "fork").await.unwrap();
    assert_eq!(first.rating, 1495);
    let mut seen = vec![];
    let mut puzzle = Some(first);
    while let Some(p) = puzzle {
        assert!(p.themes.iter().any(|t| t == "fork"), "{:?}", p.themes);
        assert!(!seen.contains(&p.id), "served twice: {}", p.id);
        seen.push(p.id.clone());
        attempt(&base, &me, &p.id, true).await;
        assert!(seen.len() <= 7);
        puzzle = next_themed(&base, &me, "fork").await;
    }
    let mut want: Vec<String> = forks.into_iter().map(|(id, _)| id).collect();
    want.sort();
    seen.sort();
    assert_eq!(seen, want);
    // Other themes are still there, and so is the unfiltered queue.
    assert!(next_themed(&base, &me, "pin").await.is_some());
    assert!(next(&base, &me).await.is_some());

    // A theme nothing has is simply empty; a malformed one is refused.
    assert!(next_themed(&base, &me, "noSuchTheme").await.is_none());
    let r = http(&base, "GET", "/api/puzzles/next?theme=a%27b", Some(&me), "").await;
    assert_eq!(r.status, 400);
    // An empty theme is no filter.
    assert!(next_themed(&base, &me, "").await.is_some());
}

#[tokio::test]
async fn streaks_count_first_tries_only_and_survive_a_reload() {
    let Some((_, base)) = with_fixture().await else {
        return;
    };
    let me = guest(&base).await;
    let streak = |current, best| PuzzleStreak { current, best };
    let a = next(&base, &me).await.unwrap();
    assert_eq!(a.streak, streak(0, 0));
    let solved_a = attempt(&base, &me, &a.id, true).await;
    assert_eq!(solved_a.streak, streak(1, 1));
    let b = next(&base, &me).await.unwrap();
    let solved_b = attempt(&base, &me, &b.id, true).await;
    assert_eq!(solved_b.streak, streak(2, 2));

    // Trying A again, failing or solving, rates nothing and keeps the streak.
    for solved in [false, true] {
        let again = attempt(&base, &me, &a.id, solved).await;
        assert!(!again.counted);
        assert_eq!(again.diff, 0);
        assert_eq!(again.rating, solved_b.rating);
        assert_eq!(again.streak, streak(2, 2));
    }

    let c = next(&base, &me).await.unwrap();
    let failed = attempt(&base, &me, &c.id, false).await;
    assert_eq!(failed.streak, streak(0, 2));
    // What the next puzzle carries is what a reloaded page shows.
    let d = next(&base, &me).await.unwrap();
    assert_eq!(d.streak, streak(0, 2));
    assert_eq!(attempt(&base, &me, &d.id, true).await.streak, streak(1, 2));
}

#[tokio::test]
async fn the_daily_puzzle_is_everyones_and_rates_only_a_first_try() {
    let Some((_, base)) = with_fixture().await else {
        return;
    };
    let (one, two) = (guest(&base).await, guest(&base).await);
    let today = format_date(today());
    let mine = daily(&base, &one, "/api/puzzles/daily").await;
    let theirs = daily(&base, &two, "/api/puzzles/daily").await;
    assert_eq!(mine.date, today);
    assert_eq!(mine.puzzle.id, theirs.puzzle.id);
    // The same through its dated link.
    let dated = daily(&base, &two, &format!("/api/puzzles/daily/{today}")).await;
    assert_eq!(dated.puzzle.id, mine.puzzle.id);
    // The fixture has puzzles in the band, so it is one of those.
    let band = DAILY_BAND.0..=DAILY_BAND.1;
    assert!(band.contains(&mine.puzzle.rating), "{}", mine.puzzle.rating);
    assert!(!mine.puzzle.tried);

    // The first try counts, like any other puzzle's; the page reloaded says
    // it has been tried, and trying again changes nothing.
    let first = attempt(&base, &one, &mine.puzzle.id, true).await;
    assert!(first.counted && first.diff != 0);
    let reloaded = daily(&base, &one, "/api/puzzles/daily").await;
    assert!(reloaded.puzzle.tried);
    assert_eq!(reloaded.puzzle.your_rating, first.rating);
    assert_eq!(reloaded.puzzle.streak, first.streak);
    let again = attempt(&base, &one, &reloaded.puzzle.id, false).await;
    assert_eq!(
        again,
        AttemptResult {
            counted: false,
            diff: 0,
            ..first
        }
    );
    // Nor does the ordinary queue hand it back.
    for _ in 0..10 {
        let p = next(&base, &one).await.unwrap();
        assert_ne!(p.id, mine.puzzle.id);
        attempt(&base, &one, &p.id, true).await;
    }

    // Someone who met the puzzle elsewhere first gets no second rating.
    let three = guest(&base).await;
    let met = attempt(&base, &three, &mine.puzzle.id, false).await;
    assert!(met.counted);
    let later = daily(&base, &three, "/api/puzzles/daily").await;
    assert!(later.puzzle.tried);
    assert!(!attempt(&base, &three, &later.puzzle.id, true).await.counted);
}

#[tokio::test]
async fn past_days_keep_their_puzzle_and_the_future_is_hidden() {
    let Some((_, base)) = with_fixture().await else {
        return;
    };
    let me = guest(&base).await;
    let mut ids = std::collections::HashSet::new();
    for day in 1..=10 {
        let path = format!("/api/puzzles/daily/2026-01-{day:02}");
        let a = daily(&base, &me, &path).await;
        let b = daily(&base, &me, &path).await;
        assert_eq!(a.puzzle.id, b.puzzle.id);
        assert_eq!(a.date, format!("2026-01-{day:02}"));
        let band = DAILY_BAND.0..=DAILY_BAND.1;
        assert!(band.contains(&a.puzzle.rating), "{}", a.puzzle.rating);
        ids.insert(a.puzzle.id);
    }
    // Five fixture puzzles are in the band; ten days don't all pick one.
    assert!(ids.len() > 1, "{ids:?}");

    for (path, status) in [
        ("/api/puzzles/daily/2026-02-30", 400),
        ("/api/puzzles/daily/yesterday", 400),
        ("/api/puzzles/daily/9999-12-31", 404),
    ] {
        assert_eq!(
            http(&base, "GET", path, Some(&me), "").await.status,
            status,
            "{path}"
        );
    }
    assert_eq!(
        http(&base, "GET", "/api/puzzles/daily", None, "")
            .await
            .status,
        401
    );
}
