use std::process::Command;

const STOCKFISH_SCRIPT: &str = "./build-stockfish.sh";

fn main() {
    Command::new(STOCKFISH_SCRIPT)
        .status()
        .expect("Failed to build stockfish");
}
