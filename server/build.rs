use build_common::command_config::CommandConfig;
use build_common::helpers::{get_stockfish_commands, get_tailwind_commands, get_trunk_commands};

fn main() {
    println!("cargo:rerun-if-changed=../client");

    let mut commands = get_stockfish_commands(true);
    commands.extend(get_tailwind_commands());
    commands.extend(get_trunk_commands());
    CommandConfig::run_build_commands(&commands);
}
