//! Ask the real coach about a fixed set of positions and check its answers.
//!
//! ```sh
//! set -a; source .env; set +a                          # CHESS_ANTHROPIC_API_KEY
//! cargo run -p server --example coach_eval             # every case
//! cargo run -p server --example coach_eval -- mate     # cases whose name contains "mate"
//! cargo run -p server --example coach_eval -- --prompts  # print what the model is sent, too
//! cargo run -p server --example coach_eval -- --dry-run  # only the prompts, no API calls
//! cargo run -p server --example coach_eval -- --no-judge  # answers only, no grading
//! cargo run -p server --example coach_eval -- --repeat 3   # each case three times
//! cargo run -p server --example coach_eval -- --regrade target/coach-eval/<run>.json
//! cargo run -p server --example coach_eval -- --compare target/coach-eval/<run>.json
//! ```
//!
//! The cases (`tests/fixtures/coach_eval.json`) are positions to explain and
//! mistakes, with real Stockfish lines; `cargo run -p server --example
//! coach_cases` rebuilds them from a local Stockfish. Each
//! is answered the way the server answers: checked with `Prompt::check` (moves
//! and pieces the prompt never showed, Markdown) and, when flagged, rewritten
//! once. What was flagged and whether the rewrite is served are printed with
//! the answer, for a human to read: the check can't catch every mistake. Every
//! case is one API call (two when rewritten; about 1¢ each on Opus 5), made in
//! parallel. Cases with `follow_ups` then ask those on the answer's thread,
//! with the Stockfish line the client would send for a move one names.
//! `CHESS_COACH_MODEL` picks the model, as for the server.
//!
//! Each answer is then graded (`judge`) by a second model call against the
//! same facts and lines, for accuracy, clarity and usefulness out of 5, with
//! every unsupported claim quoted: a judge is itself a language model, so the
//! run prints the quotes to read. `CHESS_JUDGE_MODEL` picks that model.
//! Every run is saved under `target/coach-eval/` and `--compare` puts an
//! earlier one beside it, which is how a prompt change is judged. Both the
//! answers and the judge vary from run to run (the same case has scored 2 and
//! 5 on the same code), so judge a change on `--repeat 3` or more and on the
//! means, not on one sample.
//!
//! `--regrade <run.json>` grades the answers of an earlier run again with
//! the current judge, and prints how the two judges differ: that is how to
//! tell whether a cheaper judge agrees with a dearer one, without paying for
//! new answers.

mod judge;

use std::{
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use futures_util::future::join_all;
use judge::{DEFAULT_JUDGE_MODEL, Judge, Judgement};
use serde::{Deserialize, Serialize};
use server::coach::Usage;
use server::coach::{
    Coach, CoachConfig, DEFAULT_MODEL, ExplainRequest, FollowUpReply, MistakeRequest, Probe,
    Prompt, mistake_prompt, prompt,
};

#[derive(Deserialize)]
struct Case {
    name: String,
    #[serde(flatten)]
    ask: Ask,
    /// Asked in order after the answer, as a student would.
    #[serde(default)]
    follow_ups: Vec<FollowUpCase>,
}

#[derive(Deserialize)]
struct FollowUpCase {
    question: String,
    /// Stockfish's line for the move the question names, as the client sends it.
    probe: Option<Probe>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Ask {
    Explain(ExplainRequest),
    Mistake(MistakeRequest),
}

const CASES: &str = include_str!("../../tests/fixtures/coach_eval.json");

/// One case's outcome, saved so runs can be compared.
#[derive(Debug, Serialize, Deserialize)]
struct CaseResult {
    name: String,
    /// Which time round this case was asked (`--repeat`).
    #[serde(default)]
    sample: usize,
    kind: String,
    text: String,
    words: usize,
    seconds: f32,
    flagged: Vec<String>,
    problems: Vec<String>,
    rewritten: bool,
    judgement: Option<Judgement>,
    follow_ups: Vec<FollowUpResult>,
    error: Option<String>,
    #[serde(default)]
    usage: Usage,
    #[serde(default)]
    judge_usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
struct FollowUpResult {
    question: String,
    text: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Run {
    when_unix: u64,
    model: String,
    judge_model: Option<String>,
    cases: Vec<CaseResult>,
    /// What the run cost: tokens per model, and the price of them.
    #[serde(default)]
    spend: Vec<Spend>,
}

/// Tokens used on one model, and what they cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Spend {
    model: String,
    input: u32,
    output: u32,
    dollars: Option<f64>,
}

/// Dollars per million tokens, input then output.
fn price(model: &str) -> Option<(f64, f64)> {
    Some(match model {
        "claude-opus-5" | "claude-opus-4-8" => (5.0, 25.0),
        "claude-sonnet-5" => (2.0, 10.0),
        "claude-haiku-4-5" => (1.0, 5.0),
        "claude-fable-5-1" | "claude-fable-5" => (10.0, 50.0),
        _ => return None,
    })
}

impl Spend {
    fn of(model: &str, usage: Usage) -> Spend {
        let dollars = price(model).map(|(input, output)| {
            f64::from(usage.input) * input / 1e6 + f64::from(usage.output) * output / 1e6
        });
        Spend {
            model: model.to_string(),
            input: usage.input,
            output: usage.output,
            dollars,
        }
    }
}

impl Run {
    fn graded(&self) -> impl Iterator<Item = (&str, &Judgement)> {
        self.cases
            .iter()
            .filter_map(|c| Some((c.name.as_str(), c.judgement.as_ref()?)))
    }

    /// A case's scores averaged over its samples: accuracy, clarity,
    /// usefulness, and how far its total ranged.
    fn case_means(&self, name: &str) -> Option<(f32, f32, f32, u32, u32)> {
        let scores: Vec<&Judgement> = self
            .graded()
            .filter(|(n, _)| *n == name)
            .map(|(_, j)| j)
            .collect();
        let n = scores.len() as f32;
        (n > 0.0).then(|| {
            let mean =
                |f: fn(&Judgement) -> u8| scores.iter().map(|j| f(j) as f32).sum::<f32>() / n;
            let totals: Vec<u32> = scores.iter().map(|j| j.total()).collect();
            (
                mean(|j| j.accuracy),
                mean(|j| j.clarity),
                mean(|j| j.usefulness),
                totals.iter().copied().min().unwrap_or(0),
                totals.iter().copied().max().unwrap_or(0),
            )
        })
    }

    /// Each case once, in order.
    fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for case in &self.cases {
            if !names.contains(&case.name.as_str()) {
                names.push(&case.name);
            }
        }
        names
    }

    /// Accuracy, clarity and usefulness averaged over the graded cases.
    fn averages(&self) -> Option<(f32, f32, f32)> {
        let scores: Vec<&Judgement> = self.graded().map(|(_, j)| j).collect();
        let n = scores.len() as f32;
        (n > 0.0).then(|| {
            let sum = |f: fn(&Judgement) -> u8| scores.iter().map(|j| f(j) as f32).sum::<f32>() / n;
            (
                sum(|j| j.accuracy),
                sum(|j| j.clarity),
                sum(|j| j.usefulness),
            )
        })
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let show_prompts = dry_run || args.iter().any(|a| a == "--prompts");
    let grade = !args.iter().any(|a| a == "--no-judge");
    let repeat: usize = flag(&args, "--repeat")
        .and_then(|n| n.parse().ok())
        .unwrap_or(1)
        .max(1);
    let compare = flag(&args, "--compare").map(PathBuf::from);
    let regrade = flag(&args, "--regrade").map(PathBuf::from);
    let paths: Vec<&str> = [compare.as_deref(), regrade.as_deref()]
        .into_iter()
        .flatten()
        .filter_map(|p| p.to_str())
        .collect();
    let filters: Vec<&String> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .filter(|a| !paths.contains(&a.as_str()))
        .filter(|a| a.parse::<usize>().is_err())
        .collect();

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
    let sample_key = key.clone();
    let coach = Coach::new(CoachConfig {
        model: model.clone(),
        per_hour: u32::MAX,
        ..CoachConfig::anthropic(key.clone())
    });
    let judge = grade.then(|| {
        let model =
            std::env::var("CHESS_JUDGE_MODEL").unwrap_or_else(|_| DEFAULT_JUDGE_MODEL.to_string());
        std::sync::Arc::new(Judge::new(key, model))
    });

    if let Some(path) = regrade {
        let Some(judge) = judge else {
            eprintln!("--regrade needs a judge; drop --no-judge");
            std::process::exit(2);
        };
        regrade_run(&path, &judge, &cases).await;
        return;
    }

    let times = if repeat > 1 {
        format!(", {repeat} times each")
    } else {
        String::new()
    };
    match &judge {
        Some(j) => println!(
            "{} cases on {model}{times}, graded by {}\n",
            cases.len(),
            j.model
        ),
        None => println!("{} cases on {model}{times}, ungraded\n", cases.len()),
    }

    // Each sample gets its own coach: one would serve a case's second ask
    // from its answer cache.
    let samples: Vec<(usize, &Case)> = (0..repeat)
        .flat_map(|sample| cases.iter().map(move |case| (sample, case)))
        .collect();
    let runs = samples.iter().map(|&(sample, case)| {
        let coach = if repeat > 1 {
            Coach::new(CoachConfig {
                model: model.clone(),
                per_hour: u32::MAX,
                ..CoachConfig::anthropic(sample_key.clone())
            })
        } else {
            coach.clone()
        };
        let judge = judge.clone();
        async move {
            let prompt = case_prompt(case);
            let started = Instant::now();
            let answer = coach.answer(prompt.clone()).await;
            let took = started.elapsed();
            // Graded against the same facts the coach was given.
            let judgement = match (judge.as_deref(), &answer) {
                (Some(judge), Ok(answer)) => Some(judge.grade(&prompt.user, &answer.text).await),
                _ => None,
            };
            // Then the follow-ups, one after another on the answer's thread.
            let mut follow_ups = Vec::new();
            if let Ok(answer) = &answer {
                let thread = coach.start_thread("eval", &prompt, answer);
                for f in &case.follow_ups {
                    let reply = coach
                        .follow_up(&thread, "eval", &f.question, f.probe.clone())
                        .await;
                    follow_ups.push((&f.question, reply));
                }
            }
            (sample, prompt, answer, took, follow_ups, judgement)
        }
    });
    let results = join_all(runs).await;

    let (mut flagged, mut rewritten, mut still, mut failed) = (0, 0, 0, 0);
    let mut run = Run {
        spend: Vec::new(),
        when_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        model: model.clone(),
        judge_model: judge.as_ref().map(|j| j.model.clone()),
        cases: Vec::new(),
    };
    for (&(_, case), (sample, prompt, answer, took, follow_ups, judgement)) in
        samples.iter().zip(results)
    {
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
                run.cases.push(CaseResult {
                    name: case.name.clone(),
                    sample,
                    usage: Usage::default(),
                    judge_usage: Usage::default(),
                    kind: kind.to_string(),
                    text: String::new(),
                    words: 0,
                    seconds: took.as_secs_f32(),
                    flagged: Vec::new(),
                    problems: Vec::new(),
                    rewritten: false,
                    judgement: None,
                    follow_ups: Vec::new(),
                    error: Some(format!("{e:?}")),
                });
                continue;
            }
        };
        flagged += usize::from(!answer.flagged.is_empty());
        rewritten += usize::from(answer.rewritten);
        still += usize::from(!answer.problems.is_empty());
        let (judgement, judge_usage) = match judgement {
            Some(Ok((j, usage))) => (Some(j), usage),
            Some(Err(e)) => {
                println!("  !! the judge failed: {e}");
                (None, Usage::default())
            }
            None => (None, Usage::default()),
        };
        println!(
            "== {}{} ({kind}) · {:.1}s · {} words{}{}",
            case.name,
            if repeat > 1 {
                format!(" #{}", sample + 1)
            } else {
                String::new()
            },
            took.as_secs_f32(),
            answer.text.split_whitespace().count(),
            if answer.rewritten {
                " · rewritten"
            } else {
                ""
            },
            match &judgement {
                Some(j) => format!(
                    " · accuracy {} clarity {} usefulness {}",
                    j.accuracy, j.clarity, j.usefulness
                ),
                None => String::new(),
            }
        );
        println!("{}", answer.text);
        for p in &answer.flagged {
            println!("  flagged: {p}");
        }
        for p in &answer.problems {
            println!("  !! still: {p}");
        }
        if let Some(j) = &judgement {
            for u in &j.unsupported {
                println!("  judge: {:?} — {}", u.quote, u.why);
            }
            if !j.notes.is_empty() {
                println!("  judge: {}", j.notes);
            }
        }
        let mut saved_follow_ups = Vec::new();
        for (question, reply) in follow_ups {
            println!("  > {question}");
            let (text, error) = match reply {
                Ok(FollowUpReply::Answer { answer, .. }) => {
                    println!("  {}", answer.text);
                    (Some(answer.text), None)
                }
                Ok(FollowUpReply::Probe { san, .. }) => {
                    failed += 1;
                    println!("  !! the fixture needs a probe for {san}");
                    (None, Some(format!("needs a probe for {san}")))
                }
                Err(e) => {
                    failed += 1;
                    println!("  !! FAILED {e:?}");
                    (None, Some(format!("{e:?}")))
                }
            };
            saved_follow_ups.push(FollowUpResult {
                question: question.clone(),
                text,
                error,
            });
        }
        println!();
        run.cases.push(CaseResult {
            name: case.name.clone(),
            sample,
            usage: answer.usage,
            judge_usage,
            kind: kind.to_string(),
            words: answer.text.split_whitespace().count(),
            text: answer.text,
            seconds: took.as_secs_f32(),
            flagged: answer.flagged,
            problems: answer.problems,
            rewritten: answer.rewritten,
            judgement,
            follow_ups: saved_follow_ups,
            error: None,
        });
    }
    println!(
        "{} cases, {} answers: {flagged} flagged at first, {rewritten} rewritten, {still} still \
         flagged, {failed} failed",
        cases.len(),
        samples.len()
    );
    let mut answers = Usage::default();
    let mut grading = Usage::default();
    for case in &run.cases {
        answers += case.usage;
        grading += case.judge_usage;
    }
    run.spend.push(Spend::of(&model, answers));
    if let Some(j) = &judge {
        run.spend.push(Spend::of(&j.model, grading));
    }
    for spend in &run.spend {
        println!(
            "{}: {} tokens in, {} out{}",
            spend.model,
            spend.input,
            spend.output,
            match spend.dollars {
                Some(d) => format!(" — ${d:.2}"),
                None => String::new(),
            }
        );
    }
    let total: f64 = run.spend.iter().filter_map(|s| s.dollars).sum();
    if total > 0.0 {
        println!("this run cost about ${total:.2}");
    }
    if let Some((accuracy, clarity, usefulness)) = run.averages() {
        let claims: usize = run.graded().map(|(_, j)| j.unsupported.len()).sum();
        let cases_with = run
            .graded()
            .filter(|(_, j)| !j.unsupported.is_empty())
            .count();
        println!(
            "graded: accuracy {accuracy:.2}, clarity {clarity:.2}, usefulness {usefulness:.2} \
             (out of 5); {claims} unsupported claims in {cases_with} {}",
            if cases_with == 1 { "answer" } else { "answers" }
        );
    }
    match save(&run) {
        Ok(path) => println!("saved to {}", path.display()),
        Err(e) => eprintln!("could not save the run: {e}"),
    }
    if let Some(path) = compare {
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| serde_json::from_str::<Run>(&text).map_err(|e| e.to_string()))
        {
            Ok(before) => print_comparison(&before, &run),
            Err(e) => eprintln!("could not read {}: {e}", path.display()),
        }
    }
}

/// The value after `flag` in `args`.
fn flag<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let at = args.iter().position(|a| a == flag)?;
    args.get(at + 1).map(String::as_str)
}

fn save(run: &Run) -> Result<PathBuf, String> {
    let dir = PathBuf::from("target/coach-eval");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", run.when_unix));
    let text = serde_json::to_string_pretty(run).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

/// An earlier run beside this one, case by case: what a prompt change did.
fn print_comparison(before: &Run, now: &Run) {
    println!(
        "\nagainst the run of {} ({})",
        before.when_unix, before.model
    );
    for name in now.names() {
        let Some(new) = now.case_means(name) else {
            continue;
        };
        let Some(old) = before.case_means(name) else {
            println!("  {name}: new");
            continue;
        };
        let change = (new.0 + new.1 + new.2) - (old.0 + old.1 + old.2);
        let arrow = if change.abs() < 0.05 {
            "="
        } else if change > 0.0 {
            "+"
        } else {
            "-"
        };
        println!(
            "  {arrow} {name}: {:.1}/{:.1}/{:.1} ({}-{}) -> {:.1}/{:.1}/{:.1} ({}-{})",
            old.0, old.1, old.2, old.3, old.4, new.0, new.1, new.2, new.3, new.4
        );
    }
    if let (Some(old), Some(new)) = (before.averages(), now.averages()) {
        println!(
            "  averages: {:.2}/{:.2}/{:.2} -> {:.2}/{:.2}/{:.2}",
            old.0, old.1, old.2, new.0, new.1, new.2
        );
    }
}

fn case_prompt(case: &Case) -> Prompt {
    match &case.ask {
        Ask::Explain(r) => prompt(r),
        Ask::Mistake(r) => mistake_prompt(r),
    }
    .unwrap_or_else(|e| panic!("{}: {e}", case.name))
}

/// Grade an earlier run's answers again with this judge, and show how the
/// two judges see the same answers.
async fn regrade_run(path: &PathBuf, judge: &Judge, cases: &[Case]) {
    let before: Run = match std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
    {
        Ok(run) => run,
        Err(e) => {
            eprintln!("could not read {}: {e}", path.display());
            std::process::exit(2);
        }
    };
    let wanted: Vec<&CaseResult> = before
        .cases
        .iter()
        .filter(|c| c.judgement.is_some() && cases.iter().any(|w| w.name == c.name))
        .collect();
    println!(
        "regrading {} answers from the run of {} with {}\n",
        wanted.len(),
        before.when_unix,
        judge.model
    );
    let prompts: Vec<String> = wanted
        .iter()
        .map(|c| {
            let case = cases.iter().find(|w| w.name == c.name).expect("the case");
            case_prompt(case).user
        })
        .collect();
    let graded = join_all(
        wanted
            .iter()
            .zip(&prompts)
            .map(|(c, prompt)| judge.grade(prompt, &c.text)),
    )
    .await;

    let mut usage = Usage::default();
    let (mut agreed, mut apart) = (0.0, 0.0);
    for (case, graded) in wanted.iter().zip(graded) {
        let old = case.judgement.as_ref().expect("a judgement");
        match graded {
            Ok((new, cost)) => {
                usage += cost;
                agreed += 1.0;
                apart += (f64::from(new.accuracy) - f64::from(old.accuracy)).abs();
                println!(
                    "  {}: accuracy {} -> {}, clarity {} -> {}, usefulness {} -> {} ({} claims -> {})",
                    case.name,
                    old.accuracy,
                    new.accuracy,
                    old.clarity,
                    new.clarity,
                    old.usefulness,
                    new.usefulness,
                    old.unsupported.len(),
                    new.unsupported.len()
                );
            }
            Err(e) => println!("  {}: the judge failed: {e}", case.name),
        }
    }
    if agreed > 0.0 {
        println!(
            "\naccuracy differs by {:.2} on average between {} and {}",
            apart / agreed,
            before.judge_model.as_deref().unwrap_or("the old judge"),
            judge.model
        );
    }
    let spend = Spend::of(&judge.model, usage);
    println!(
        "{}: {} tokens in, {} out{}",
        spend.model,
        spend.input,
        spend.output,
        match spend.dollars {
            Some(d) => format!(" — ${d:.2}"),
            None => String::new(),
        }
    );
}
