use std::env;
use std::path::Path;
use std::process::Command;

const STOCKFISH_SCRIPT: &str = "./build-stockfish.sh";
const TRUNK_BUILD: &str = "trunk";
const TAILWIND_CSS_COMMAND: &str = "npx";

// Separate the command arguments from the command itself for better clarity
const TAILWIND_CSS_ARGS: [&str; 5] = [
    "tailwindcss",
    "-i",
    "./styles/input.css",
    "-o",
    "./styles/output.css",
];
const TRUNK_BUILD_ARGS: [&str; 1] = ["build"];

fn run_command(command: &str, args: &[&str], working_dir: &Path) -> bool {
    Command::new(command)
        .args(args)
        .current_dir(working_dir)
        .status()
        .expect("Failed to execute command")
        .success()
}

fn main() {
    // Get the directory of the Cargo.toml for the `client` crate
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get manifest directory");

    let current_dir = Path::new(&manifest_dir);

    // Convert it to a Path and get the parent directory to navigate to the workspace root
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .expect("Failed to get workspace root");

    // Execute the STOCKFISH_SCRIPT
    if !run_command(STOCKFISH_SCRIPT, &[], current_dir) {
        panic!("Failed to build stockfish");
    }

    // Execute the TRUNK_BUILD command
    if !run_command(TRUNK_BUILD, &TRUNK_BUILD_ARGS, workspace_root) {
        panic!("Failed to run trunk build");
    }

    // Execute the TAILWIND_CSS command
    if !run_command(TAILWIND_CSS_COMMAND, &TAILWIND_CSS_ARGS, workspace_root) {
        panic!("Failed to run tailwind css command");
    }
}
