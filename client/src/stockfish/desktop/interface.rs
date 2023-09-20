use crate::arrows::Arrows;
use crate::stockfish::client::{process_output, MOVES};
use crate::stockfish::stockfish_interface::StockfishInterface;
use async_process::{Child, Command, Stdio};
use async_std::io::BufReader;
use async_std::prelude::*;
use async_trait::async_trait;
use dioxus::prelude::*;
use futures::executor::block_on;
use std::io::Result;

struct DesktopStockfish;

#[async_trait]
impl StockfishInterface for DesktopStockfish {
    type Process = Child;

    async fn run_stockfish(&self) -> Result<Self::Process> {
        let mut cmd = Command::new("nice");
        cmd.args(["-n", "19", "client/Stockfish/src/stockfish"])
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .kill_on_drop(true);
        cmd.spawn()
    }

    async fn send_command(&self, process: &UseRef<Option<Self::Process>>, command: &str) {
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

    async fn update_analysis_arrows(&self, arrows: UseRef<Arrows>, process: UseRef<Option<Self::Process>>) {
        let stdout = process.with_mut(|process| process.as_mut().unwrap().stdout.take().unwrap());
        let mut lines = BufReader::new(stdout).lines();
        let mut evals = vec![f64::NEG_INFINITY; MOVES];
        while let Some(Ok(output)) = &lines.next().await {
            process_output(output, &mut evals, &arrows).await;
        }
    }
}
