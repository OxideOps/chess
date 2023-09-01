use build_common::command_config::CommandConfig;

fn main() {
    println!("cargo:rerun-if-changed=../client/styles");
    println!("cargo:rerun-if-changed=../client/Stockfish");

    let commands = vec![CommandConfig {
        program: "trunk",
        args: Some(&["build"]),
    }];

    CommandConfig::run_build_commands(&commands);
}
