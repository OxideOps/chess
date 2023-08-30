use build_common::command_config::CommandConfig;
use std::process::Command;

const COMMANDS: &[CommandConfig] = &[
    CommandConfig {
        cmd: "./build-stockfish.sh",
        args: None,
        dir: None,
    },
    CommandConfig {
        cmd: "npx",
        args: Some(&[
            "tailwindcss",
            "-i",
            "./styles/input.css",
            "-o",
            "./styles/output.css",
        ]),
        dir: None,
    },
];

fn run_command(config: &CommandConfig) -> Result<(), String> {
    let mut command = Command::new(config.cmd);

    if let Some(args) = config.args {
        command.args(args);
    }

    if let Some(directory) = config.dir {
        command.current_dir(directory);
    }

    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(format!(
            "Command '{}' did not finish successfully.",
            config.cmd
        )),
        Err(e) => Err(format!("Failed to execute command '{}': {}", config.cmd, e)),
    }
}

fn main() {
    for command_config in COMMANDS {
        if let Err(e) = run_command(command_config) {
            eprintln!("{}", e);
            return;
        }
    }
}
