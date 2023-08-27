use build_common::command_config::CommandConfig;
use std::process::Command;

fn get_commands() -> Vec<CommandConfig> {
    let is_wasm_target = std::env::var("TARGET").map_or(false, |target| target.contains("wasm32"));
    let stockfish_args = if is_wasm_target {
        Some(&["--wasm"][..])
    } else {
        None
    };
    println!("{:?}", stockfish_args);

    vec![
        CommandConfig {
            program: "./build-stockfish.sh",
            args: stockfish_args,
        },
        CommandConfig {
            program: "./build-tailwind.sh",
            args: None,
        },
    ]
}

fn main() {
    println!("cargo:rerun-if-changed=./styles");
    println!("cargo:rerun-if-changed=./Stockfish");

    for cmd_cfg in get_commands() {
        let mut cmd = Command::new(cmd_cfg.program);

        if let Some(args) = cmd_cfg.args {
            cmd.args(args);
        }
        assert!(
            cmd.status()
                .unwrap_or_else(|_| panic!("failed to execute {}", cmd_cfg.program))
                .success(),
            "termination was not successful for {}",
            cmd_cfg.program
        );
    }
}
