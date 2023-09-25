use dioxus::prelude::{UseAsyncLock, UseLock};

pub trait Stockfish {
    type Process;
    async fn send_command(&self, process: &UseAsyncLock<Option<Self::Process>>, command: &str);
    async fn run_stockfish(&self) -> Result<Self::Process, String>;
    async fn run_stockfish(&self) -> Result<Self::Process>;
    async fn update_analysis_arrows(&self, arrows: UseLock<Arrows>, process: UseAsyncLock<Option<Self::Process>>);
}