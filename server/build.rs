use build_common::{
    get_stockfish_commands, get_tailwind_commands, get_trunk_commands, CommandConfig,
};

fn main() {
    println!("cargo:rerun-if-changed=../client");

    let mut commands = get_stockfish_commands(true);
    commands.extend(get_tailwind_commands());
    commands.extend(get_trunk_commands());
    CommandConfig::run_build_commands(&commands);
}
