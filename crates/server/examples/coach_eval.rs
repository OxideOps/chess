//! Ask the real coach about a fixed set of positions and check its answers.
//!
//! ```sh
//! set -a; source .env; set +a                          # CHESS_ANTHROPIC_API_KEY
//! cargo run -p server --example coach_eval             # every case
//! cargo run -p server --example coach_eval -- mate     # cases whose name contains "mate"
//! cargo run -p server --example coach_eval -- --prompts  # print what the model is sent, too
//! cargo run -p server --example coach_eval -- --dry-run  # only the prompts, no API calls
//! ```
//!
//! The cases (`tests/fixtures/coach_eval.json`) are positions to explain and
//! drill mistakes, with real Stockfish lines (Stockfish 17.1, depth 22). Each
//! is answered the way the server answers: checked with `Prompt::check` (moves
//! and pieces the prompt never showed, Markdown) and, when flagged, rewritten
//! once. What was flagged and whether the rewrite is served are printed with
//! the answer, for a human to read: the check can't catch every mistake. Every
//! case is one API call (two when rewritten; about 1¢ each on Opus 5), made in
//! parallel. `CHESS_COACH_MODEL` picks the model, as for the server.

use std::time::Instant;

use futures_util::future::join_all;
use serde::Deserialize;
use server::coach::{
    Coach, CoachConfig, DEFAULT_MODEL, ExplainRequest, MistakeRequest, Prompt, mistake_prompt,
    prompt,
};

#[derive(Deserialize)]
struct Case {
    name: String,
    #[serde(flatten)]
    ask: Ask,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Ask {
    Explain(ExplainRequest),
    Mistake(MistakeRequest),
}

const CASES: &str = include_str!("../tests/fixtures/coach_eval.json");

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let show_prompts = dry_run || args.iter().any(|a| a == "--prompts");
    let filters: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();

    let cases: Vec<Case> = serde_json::from_str(CASES).expect("the cases parse");
    let cases: Vec<Case> = cases
        .into_iter()
        .filter(|c| filters.is_empty() || filters.iter().any(|f| c.name.contains(f.as_str())))
        .collect();
    if dry_run {
        for case in &cases {
            println!("---- prompt: {}\n{}\n", case.name, case_prompt(case).user);
        }
        return;
    }

    let Ok(key) = std::env::var("CHESS_ANTHROPIC_API_KEY") else {
        eprintln!("CHESS_ANTHROPIC_API_KEY is not set (try: set -a; source .env; set +a)");
        std::process::exit(2);
    };
    let model = std::env::var("CHESS_COACH_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let coach = Coach::new(CoachConfig {
        model: model.clone(),
        per_hour: u32::MAX,
        ..CoachConfig::anthropic(key)
    });

    println!("{} cases on {model}\n", cases.len());

    let runs = cases.iter().map(|case| {
        let coach = coach.clone();
        async move {
            let prompt = case_prompt(case);
            let started = Instant::now();
            let answer = coach.answer(prompt.clone()).await;
            (prompt, answer, started.elapsed())
        }
    });
    let results = join_all(runs).await;

    let (mut flagged, mut rewritten, mut still, mut failed) = (0, 0, 0, 0);
    for (case, (prompt, answer, took)) in cases.iter().zip(results) {
        let kind = match case.ask {
            Ask::Explain(_) => "explain",
            Ask::Mistake(_) => "mistake",
        };
        if show_prompts {
            println!("---- prompt: {}\n{}\n", case.name, prompt.user);
        }
        let answer = match answer {
            Ok(answer) => answer,
            Err(e) => {
                failed += 1;
                println!("== {} ({kind}): FAILED {e:?}\n", case.name);
                continue;
            }
        };
        flagged += usize::from(!answer.flagged.is_empty());
        rewritten += usize::from(answer.rewritten);
        still += usize::from(!answer.problems.is_empty());
        println!(
            "== {} ({kind}) · {:.1}s · {} words{}",
            case.name,
            took.as_secs_f32(),
            answer.text.split_whitespace().count(),
            if answer.rewritten {
                " · rewritten"
            } else {
                ""
            }
        );
        println!("{}", answer.text);
        for p in &answer.flagged {
            println!("  flagged: {p}");
        }
        for p in &answer.problems {
            println!("  !! still: {p}");
        }
        println!();
    }
    println!(
        "{} cases: {flagged} flagged at first, {rewritten} rewritten, {still} still flagged, \
         {failed} failed",
        cases.len()
    );
}

fn case_prompt(case: &Case) -> Prompt {
    match &case.ask {
        Ask::Explain(r) => prompt(r),
        Ask::Mistake(r) => mistake_prompt(r),
    }
    .unwrap_or_else(|e| panic!("{}: {e}", case.name))
}
