use crate::arrows::Arrows;
use anyhow::Result;
use async_process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use async_std::task::block_on;
use async_std::{io::BufReader, prelude::*};
use chess::moves::Move;
use dioxus::hooks::UseRef;
use regex::Regex;

const MOVES: usize = 3;

fn get_info<'a>(output: &'a str, key: &'a str) -> Option<&'a str> {
    let re = Regex::new(&format!(r"{key} (\S+)")).unwrap();
    re.captures(output)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
}

pub async fn run_stockfish() -> Result<Child> {
    let mut cmd = Command::new("../Stockfish/src/stockfish");
    cmd.stdout(Stdio::piped()).stdin(Stdio::piped());

    log::info!("Starting Stockfish");
    let mut child = cmd.spawn()?;

    child.stdin.as_mut().unwrap().write_all(b"uci\n").await?;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&format!("setoption name MultiPV value {MOVES}\n").into_bytes())
        .await?;

    Ok(child)
}

pub async fn update_position(fen_str: String, process: UseRef<Option<Child>>) {
    process.with_mut(|option| {
        if let Some(process) = option {
            let stdin = process.stdin.as_mut().unwrap();
            block_on(stdin.write_all(&format!("position fen {fen_str}\n").into_bytes()))
                .expect("Failed to send command to stockfish");
            block_on(stdin.write_all(b"go\n")).expect("Failed to send command to stockfish");
        }
    });
}

pub async fn update_analysis_arrows(arrows: &UseRef<Arrows>, stdout: ChildStdout) {
    let mut lines = BufReader::new(stdout).lines();

    arrows.with_mut(|arrows| *arrows = Arrows::new(vec![Move::default(); MOVES]));

    while let Some(Ok(output)) = lines.next().await {
        if let Some(i) = get_info(&output, "multipv") {
            let i = i.parse::<usize>().unwrap() - 1;
            if let Some(move_str) = get_info(&output, " pv") {
                arrows.with_mut(|arrows| arrows.set(i, Move::from_lan(move_str).unwrap()));
            }
            // if let Some(eval) = get_info(&output, "score cp") {
            //     best_lines[i].eval = eval.parse().unwrap();
            // }
        }
    }
}
