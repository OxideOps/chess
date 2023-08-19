use crate::arrows::Arrows;
use crate::stockfish_client::{process_output, MOVES};
use async_process::{Child, Command, Stdio};
use async_std::io::BufReader;
use async_std::prelude::*;
use dioxus::prelude::*;
use futures::executor::block_on;
use std::io::Result;

pub type Process = Child;

pub async fn send_command(process: &UseRef<Option<Process>>, command: &str) {
    if let Some(process) = &mut *process.write() {
        block_on(
            process
                .stdin
                .as_mut()
                .unwrap()
                .write_all(&format!("{command}\n").into_bytes()),
        )
        .expect("Failed to send stockfish command")
    }
}

pub async fn run_stockfish() -> Result<Process> {
    let mut cmd = Command::new("nice");
    cmd.args(["-n", "19", "client/Stockfish/src/stockfish"])
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .kill_on_drop(true);
    cmd.spawn()
}

pub async fn update_analysis_arrows(arrows: UseRef<Arrows>, process: UseRef<Option<Process>>) {
    let stdout = process.with_mut(|process| process.as_mut().unwrap().stdout.take().unwrap());
    let mut lines = BufReader::new(stdout).lines();
    let mut evals = vec![f64::NEG_INFINITY; MOVES];
    while let Some(Ok(output)) = &lines.next().await {
        process_output(output, &mut evals, &arrows).await;
    }
}
