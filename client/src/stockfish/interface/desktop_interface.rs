use anyhow::Result;
use async_process::{Child, Command, Stdio};
use async_std::{io::BufReader, prelude::*};
use chess::Game;
use dioxus::prelude::*;

use crate::{
    arrows::Arrows,
    stockfish::{
        core::{process_output, MOVES},
        Eval,
    },
};

pub(crate) type Process = Child;

pub(crate) async fn send_command(process: &mut Process, command: &str) {
    process
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&format!("{command}\n").into_bytes())
        .await
        .expect("Failed to send stockfish command")
}

pub(crate) async fn run_stockfish() -> Result<Process> {
    let mut cmd = Command::new("client/Stockfish/src/stockfish");
    cmd.stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .kill_on_drop(true);
    Ok(cmd.spawn()?)
}

pub(crate) async fn update_analysis_arrows(
    arrows: &UseLock<Arrows>,
    process: &UseAsyncLock<Option<Process>>,
    eval_hook: &UseSharedState<Eval>,
    game: &UseSharedState<Game>,
) {
    let stdout = process
        .write()
        .await
        .as_mut()
        .unwrap()
        .stdout
        .take()
        .unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let mut scores = vec![f64::NEG_INFINITY; MOVES];
    while let Some(Ok(output)) = &lines.next().await {
        process_output(output, &mut scores, arrows, eval_hook, game).await;
    }
}
