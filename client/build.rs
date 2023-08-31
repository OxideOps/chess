use build_common::command_config::CommandConfig;
use std::process::Command;

const COMMANDS: &[CommandConfig] = &[
    CommandConfig {
        program: "./build-stockfish.sh",
        args: None,
    },
    CommandConfig {
        program: "./build-tailwind.sh",
        args: None,
    },
];

fn main() {
    println!("cargo:rerun-if-changed=./styles");
    println!("cargo:rerun-if-changed=./Stockfish");

    for cmd_cfg in COMMANDS {
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
