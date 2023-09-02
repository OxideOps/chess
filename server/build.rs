use build_common::command_config::CommandConfig;
use std::{fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=../client/styles");
    println!("cargo:rerun-if-changed=../client/Stockfish");

    let trunk_path = fs::canonicalize("..").unwrap();
    let tailwind_program = fs::canonicalize("../client/build-tailwind.sh").unwrap();

    let commands = vec![
        CommandConfig {
            program: &tailwind_program,
            args: None,
            dir: None,
        },
        CommandConfig {
            program: Path::new("trunk"),
            args: Some(&["build"]),
            dir: Some(&trunk_path),
        },
    ];

    CommandConfig::run_build_commands(&commands);
}
