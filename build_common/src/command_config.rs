use std::{path::PathBuf, process::Command};

#[derive(Debug)]
pub struct CommandConfig {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub dir: Option<PathBuf>,
}

impl CommandConfig {
    pub fn run_build_commands(commands: &[CommandConfig]) {
        for cmd_cfg in commands {
            let mut cmd = Command::new(&cmd_cfg.program);

            if let Some(dir) = &cmd_cfg.dir {
                cmd.current_dir(dir);
            }

            cmd.args(&cmd_cfg.args);

            assert!(
                cmd.status()
                    .unwrap_or_else(|_| panic!("failed to execute {:?}", cmd_cfg.program))
                    .success(),
                "termination was not successful for {:?}",
                cmd_cfg.program
            );
        }
    }
}
