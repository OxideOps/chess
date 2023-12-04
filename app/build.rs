mod build_utils;

use std::env;

use build_utils::*;

fn main() {
    if env::var("TARGET").map_or(false, |target| target.contains("wasm32")) {
        return;
    }
    dotenvy::dotenv().ok();
    #[cfg(feature = "ssr")]
    migration::run();

    let mut commands = get_tailwind_commands();
    if cfg!(feature = "ssr") {
        commands.extend(get_stockfish_commands(true));
        commands.extend(get_trunk_commands());
    } else {
        commands.extend(get_stockfish_commands(false));
    }
    CommandConfig::run_build_commands(&commands);
}
