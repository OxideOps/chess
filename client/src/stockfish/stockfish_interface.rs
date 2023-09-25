use dioxus::prelude::{UseAsyncLock, UseLock};
use crate::arrows::Arrows;
pub trait Stockfish {
    type Process;
    type Error;
    async fn send_command(&self, process: &UseAsyncLock<Option<Self::Process>>, command: &str);
    async fn run_stockfish(&self) -> Result<Self::Process, Self::Error>;
    async fn update_analysis_arrows(&self, arrows: UseLock<Arrows>, process: UseAsyncLock<Option<Self::Process>>);
}