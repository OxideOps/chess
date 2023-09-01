use std::process::Command;

pub struct CommandConfig {
    pub program: &'static str,
    pub args: Option<&'static [&'static str]>,
}

impl CommandConfig {
    pub fn run_build_commands() {
        println!("cargo:rerun-if-changed=../client/styles");
        println!("cargo:rerun-if-changed=../client/Stockfish");

        let is_wasm_target =
            std::env::var("TARGET").map_or(false, |target| target.contains("wasm32"));
        let stockfish_args = if is_wasm_target {
            Some(&["--wasm"][..])
        } else {
            None
        };
        println!("{:?}", stockfish_args);

        let commands = vec![
            Self {
                program: "../build_common/build-stockfish.sh",
                args: stockfish_args,
            },
            Self {
                program: "../build_common/build-tailwind.sh",
                args: None,
            },
        ];

        for cmd_cfg in commands {
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
}
