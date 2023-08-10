use anyhow::Result;
use async_process::{Command, Stdio};
use async_std::{io::BufReader, prelude::*};
use regex::Regex;

const MOVES: usize = 3;

fn get_info<'a>(output: &'a str, key: &'a str) -> Option<&'a str> {
    let re = Regex::new(&*format!(r"{key} (\S+)")).unwrap();
    re.captures(output)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
}

#[derive(Default, Debug)]
struct Line {
    move_str: String,
    eval: i32,
}

async fn run_stockfish() -> Result<()> {
    let mut best_lines: [Line; MOVES] = Default::default();
    let mut cmd = Command::new("Stockfish/src/stockfish");

    cmd.stdout(Stdio::piped()).stdin(Stdio::piped());

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().expect("No stdout");
    let mut stdin = child.stdin.take().expect("No stdin");
    let mut lines = BufReader::new(stdout).lines();

    stdin.write_all(b"uci\n").await?;
    stdin
        .write_all(&format!("setoption name MultiPV value {MOVES}\n").into_bytes())
        .await?;
    stdin.write_all(b"go\n").await?;

    while let Some(Ok(output)) = lines.next().await {
        if let Some(i) = get_info(&output, "multipv") {
            let i = i.parse::<usize>().unwrap() - 1;
            if let Some(move_str) = get_info(&output, " pv") {
                best_lines[i].move_str = move_str.into();
            }
            if let Some(eval) = get_info(&output, "score cp") {
                best_lines[i].eval = eval.parse().unwrap();
            }
            let line = &best_lines[i];
            println!("{line:?}");
        }
    }

    Ok(())
}
