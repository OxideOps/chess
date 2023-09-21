use crate::command_config::CommandConfig;
use std::env;
use std::path::PathBuf;

fn get_project_root() -> PathBuf {
    PathBuf::from(
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap(),
    )
}
fn get_client_path() -> PathBuf {
    get_project_root().join("client")
}

pub fn get_tailwind_commands() -> Vec<CommandConfig> {
    let client = get_client_path();
    let tailwindcss_name = if cfg!(windows) {
        "tailwindcss.cmd"
    } else {
        "tailwindcss"
    };
    let tailwindcss_path = client.join("node_modules/.bin").join(tailwindcss_name);
    let mut commands = if !tailwindcss_path.exists() {
        vec![CommandConfig {
            program: "npm".into(),
            args: vec!["install".into()],
            dir: Some(client.clone()),
        }]
    } else {
        vec![]
    };

    commands.push(CommandConfig {
        program: tailwindcss_path,
        args: vec![
            "-i".into(),
            client.join("styles/input.css").to_string_lossy().into(),
            "-o".into(),
            client.join("styles/output.css").to_string_lossy().into(),
        ],
        dir: Some(client),
    });

    commands
}

pub fn get_stockfish_commands(wasm: bool) -> Vec<CommandConfig> {
    if cfg!(unix) {
        vec![CommandConfig {
            program: get_client_path().join("build-stockfish.sh"),
            args: if wasm { vec![] } else { vec!["--wasm".into()] },
            dir: None,
        }]
    } else {
        vec![]
    }
}

pub fn get_trunk_commands() -> Vec<CommandConfig> {
    vec![CommandConfig {
        program: PathBuf::from("trunk"),
        args: vec!["build".into()],
        dir: Some(PathBuf::from(get_project_root())),
    }]
}
