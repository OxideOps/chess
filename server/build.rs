use build_common::command_config::CommandConfig;
use std::{fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=../client/styles");
    println!("cargo:rerun-if-changed=../client/Stockfish");

    let commands = vec![
        CommandConfig {
            program: fs::canonicalize("./build-tailwind.sh").unwrap(),
            args: None,
            dir: None,
        },
        CommandConfig {
            program: PathBuf::from("trunk"),
            args: Some(&["build"]),
            dir: Some(fs::canonicalize("..").unwrap()),
        },
    ];

    CommandConfig::run_build_commands(&commands);
}
