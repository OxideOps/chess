use build_common::command_config::CommandConfig;
use std::process::Command;

const COMMANDS: &[CommandConfig] = &[
    CommandConfig {
        program: "./build-stockfish.sh",
        args: None,
    },
    CommandConfig {
        program: "npx",
        args: Some(&[
            "tailwindcss",
            "-i",
            "./styles/input.css",
            "-o",
            "./styles/output.css",
        ]),
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
            cmd.status().expect("failed to execute process").success(),
            "termination was not successful"
        );
    }
}
