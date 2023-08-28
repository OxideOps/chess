use std::process::Command;

const COMMANDS: &[(&str, &[&str])] = &[
    ("./build-stockfish.sh", &[]),
    (
        "npx",
        &[
            "tailwindcss",
            "-i",
            "../styles/input.css",
            "-o",
            "../styles/output.css",
        ],
    ),
];

fn run_command(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd).args(args).status().unwrap().success()
}

fn main() {
    for &(cmd, args) in COMMANDS {
        if !run_command(cmd, args) {
            eprintln!("Command '{}' failed!", cmd);
            return;
        }
    }
}
