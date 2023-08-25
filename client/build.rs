use std::process::Command;

const SCRIPTS: [&str; 2] = [
    "./build-stockfish.sh",
    "./build-npx.sh",
    //"./build-trunk.sh"
];

fn run_script(command: &str) -> bool {
    Command::new(command)
        .status()
        .unwrap_or_else(|err| panic!("Failed to execute '{command}'. {err}"))
        .success()
}

fn main() {
    for script in SCRIPTS.iter() {
        if !run_script(script) {
            panic!("Failed to run script: '{}'", script);
        }
    }
}
