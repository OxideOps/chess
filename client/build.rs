use std::env;
use std::path::Path;
use std::process::Command;

const STOCKFISH_SCRIPT: &str = "./build-stockfish.sh";
const TRUNK_COMMAND: &str = "trunk";
const TAILWIND_CSS_COMMAND: &str = "npx";

// Separate the command arguments from the command itself for better clarity
const TAILWIND_CSS_ARGS: [&str; 5] = [
    "tailwindcss",
    "-i",
    "./styles/input.css",
    "-o",
    "./styles/output.css",
];
const TRUNK_COMMAND_ARGS: [&str; 1] = ["build"];

fn run_command(command: &str, args: &[&str], working_dir: &Path) -> bool {
    Command::new(command)
        .args(args)
        .current_dir(working_dir)
        .status()
        .expect("Failed to execute command")
        .success()
}

fn main() {
    let client_dir = env::current_dir().unwrap();
    let root_dir = client_dir.parent().unwrap();

    // Execute the STOCKFISH_SCRIPT
    if !run_command(STOCKFISH_SCRIPT, &[], &client_dir) {
        panic!("Failed to build stockfish");
    }

    // Execute the TRUNK_BUILD command
    if !run_command(TRUNK_COMMAND, &TRUNK_COMMAND_ARGS, root_dir) {
        panic!("Failed to run trunk build");
    }

    // Execute the TAILWIND_CSS command
    if !run_command(TAILWIND_CSS_COMMAND, &TAILWIND_CSS_ARGS, root_dir) {
        panic!("Failed to run tailwind css command");
    }
}
