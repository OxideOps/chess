use std::process::Command;

const STOCKFISH_SCRIPT: &str = "./build-stockfish.sh";
const TRUNK_SCRIPT: &str = "./build-trunk.sh";
const TAILWIND_SCRIPT: &str = "./build-npx.sh";

fn run_script(command: &str) -> bool {
    Command::new(command)
        .status()
        .unwrap_or_else(|err| panic!("Failed to execute '{command}'. {err}"))
        .success()
}

fn main() {
    // Execute the STOCKFISH_SCRIPT
    if !run_script(STOCKFISH_SCRIPT) {
        panic!("Failed to build stockfish");
    }

    // Execute the TAILWIND_CSS command
    if !run_script(TAILWIND_SCRIPT) {
        panic!("Failed to run tailwind css command");
    }

    // Execute the TRUNK_BUILD command
    if !run_script(TRUNK_SCRIPT) {
        panic!("Failed to run trunk build");
    }
}
