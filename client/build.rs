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
    for cmd_cfg in COMMANDS {
        if !Command::new(cmd_cfg.program)
            .status()
            .expect("failed to execute process")
            .success()
        {
            panic!("termination was not successful")
        }
    }
}
