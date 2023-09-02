use build_common::command_config::CommandConfig;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=../client/styles");
    println!("cargo:rerun-if-changed=../client/Stockfish");

    let is_wasm_target = env::var("TARGET").map_or(false, |target| target.contains("wasm32"));

    let commands = vec![
        CommandConfig {
            program: fs::canonicalize("./build-stockfish.sh").unwrap(),
            args: if is_wasm_target {
                Some(&["--wasm"])
            } else {
                None
            },
            dir: None,
        },
        CommandConfig {
            program: fs::canonicalize("./build-tailwind.sh").unwrap(),
            args: None,
            dir: None,
        },
    ];

    CommandConfig::run_build_commands(&commands)
}
