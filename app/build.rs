use std::env;

use build_common::{get_stockfish_commands, get_tailwind_commands, CommandConfig};

fn main() {
    println!("cargo:rerun-if-changed=../app/styles");
    println!("cargo:rerun-if-changed=../app/Stockfish");

    if env::var("TARGET").map_or(false, |target| target.contains("wasm32")) {
        return;
    }

    let mut commands = get_tailwind_commands();
    commands.extend(get_stockfish_commands(false));
    CommandConfig::run_build_commands(&commands)
}
