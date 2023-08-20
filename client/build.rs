use std::process::Command;

const STOCKFISH_SCRIPT: &str = "./build-stockfish.sh";

fn main() {
    let status = Command::new(STOCKFISH_SCRIPT)
        .status()
        .expect("Failed to execute script.");

    if !status.success() {
        panic!("{STOCKFISH_SCRIPT} did not run successfully.");
    }
}
