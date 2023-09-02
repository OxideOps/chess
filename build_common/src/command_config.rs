use std::{path::Path, process::Command};

pub struct CommandConfig<'a> {
    pub program: &'a Path,
    pub args: Option<&'a [&'a str]>,
    pub dir: Option<&'a Path>,
}

impl CommandConfig<'_> {
    pub fn run_build_commands(commands: &[CommandConfig]) {
        for cmd_cfg in commands {
            let mut cmd = Command::new(cmd_cfg.program);

            if let Some(dir) = cmd_cfg.dir {
                cmd.current_dir(dir);
            }

            if let Some(args) = cmd_cfg.args {
                cmd.args(args);
            }

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
