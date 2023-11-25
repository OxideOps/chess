use std::env;

use build_common::{
    get_stockfish_commands, get_tailwind_commands, get_trunk_commands, CommandConfig,
};

fn main() {
    println!("cargo:rerun-if-changed=../app/styles");
    println!("cargo:rerun-if-changed=../app/Stockfish");

    if env::var("TARGET").map_or(false, |target| target.contains("wasm32")) {
        return;
    }

    let mut commands = get_tailwind_commands();
    if env::var("CARGO_FEATURE_SSR").is_ok() {
        commands.extend(get_trunk_commands());
        commands.extend(get_stockfish_commands(true));
    } else {
        commands.extend(get_stockfish_commands(false));
    }
    CommandConfig::run_build_commands(&commands)
}
