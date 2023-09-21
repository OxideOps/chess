use build_common::{get_stockfish_commands, get_tailwind_commands, CommandConfig};
use std::env;

fn main() {
    println!("cargo:rerun-if-changed=../client/styles");
    println!("cargo:rerun-if-changed=../client/Stockfish");

    if env::var("TARGET").map_or(false, |target| target.contains("wasm32")) {
        return;
    }

    let mut commands = get_tailwind_commands();
    commands.extend(get_stockfish_commands(false));
    CommandConfig::run_build_commands(&commands)
}
