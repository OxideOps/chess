use std::process::Command;

fn main() {
    let status = Command::new("./build-stockfish.sh")
        .status()
        .expect("Failed to execute script.");

    if !status.success() {
        panic!("The script did not run successfully.");
    }
}
