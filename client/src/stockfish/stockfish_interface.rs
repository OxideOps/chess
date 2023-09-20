use async_trait::async_trait;
use dioxus::prelude::*;

use crate::arrows::Arrows;

#[async_trait]
pub trait StockfishInterface {
    type Process;
    async fn send_command(&self, process: &UseRef<Option<Self::Process>>, command: &str);
    async fn run_stockfish(&self) -> Result<Self::Process, String>;
    async fn update_analysis_arrows(&self, arrows: UseRef<Arrows>, process: UseRef<Option<Self::Process>>);
}
