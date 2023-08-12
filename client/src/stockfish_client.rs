use crate::arrows::Arrows;
use async_process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use async_std::task::block_on;
use async_std::{io::BufReader, prelude::*};
use chess::moves::Move;
use dioxus::hooks::UseRef;
use regex::Regex;
use std::io::Result;

const MOVES: usize = 3;

fn get_info<'a>(output: &'a str, key: &'a str) -> Option<&'a str> {
    let re = Regex::new(&format!(r"{key} (\S+)")).unwrap();
    re.captures(output)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
}

async fn send_command(stdin: &mut ChildStdin, command: &str) -> Result<()> {
    stdin.write_all(&format!("{command}\n").into_bytes()).await
}

pub async fn run_stockfish() -> Result<Child> {
    let mut cmd = Command::new("../Stockfish/src/stockfish");
    cmd.stdout(Stdio::piped()).stdin(Stdio::piped());

    log::info!("Starting Stockfish");
    let mut child = cmd.spawn()?;
    let stdin = child.stdin.as_mut().unwrap();

    send_command(stdin, "uci").await?;
    send_command(stdin, &format!("setoption name MultiPV value {MOVES}")).await?;

    Ok(child)
}

pub async fn update_position(fen_str: String, process: UseRef<Option<Child>>) -> Result<()> {
    process.with_mut(|option| -> Result<()> {
        if let Some(process) = option {
            let stdin = process.stdin.as_mut().unwrap();
            block_on(send_command(stdin, "stop"))?;
            block_on(send_command(stdin, &format!("position fen {fen_str}")))?;
            block_on(send_command(stdin, "go"))?;
        }
        Ok(())
    })?;
    Ok(())
}

pub async fn update_analysis_arrows(arrows: &UseRef<Arrows>, stdout: ChildStdout) {
    let mut lines = BufReader::new(stdout).lines();
    arrows.set(Arrows::new(vec![Move::default(); MOVES]));
    while let Some(Ok(output)) = lines.next().await {
        if let Some(i) = get_info(&output, "multipv") {
            let i = i.parse::<usize>().unwrap() - 1;
            if let Some(move_str) = get_info(&output, " pv") {
                arrows.with_mut(|arrows| arrows.set(i, Move::from_lan(move_str).unwrap()));
            }
        }
    }
}
