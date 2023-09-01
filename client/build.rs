use build_common::command_config::CommandConfig;

fn main() {
    let is_wasm_target = std::env::var("TARGET").map_or(false, |target| target.contains("wasm32"));

    let commands = vec![
        CommandConfig {
            program: "./build-stockfish.sh",
            args: if is_wasm_target {
                Some(&["--wasm"])
            } else {
                None
            },
        },
        CommandConfig {
            program: "./build-tailwind.sh",
            args: None,
        },
    ];
    CommandConfig::run_build_commands(&commands)
}
