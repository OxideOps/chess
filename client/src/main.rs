use clap::Parser;
use client::app::App;

/// Chess program
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// One of TRACE, DEBUG, INFO, WARN, or ERROR
    #[arg(short, long, default_value = "DEBUG")]
    log_level: log::LevelFilter,
}

pub fn main() {
    dioxus_logger::init(Args::parse().log_level).expect("Failed to initialize dioxus logger");

    #[cfg(target_arch = "wasm32")]
    {
        log::info!("web launching");
        dioxus_web::launch(App);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
        const WINDOW_SIZE: u32 = 800;
        log::info!("desktop launching");
        dioxus_desktop::launch_cfg(
            App,
            Config::new().with_window(WindowBuilder::new().with_title("Chess").with_inner_size(
                LogicalSize {
                    width: WINDOW_SIZE,
                    height: WINDOW_SIZE,
                },
            )),
        );
    }
}
