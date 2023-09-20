use client::components::App;
use common::args::*;
use dioxus_fullstack::prelude::*;

pub fn main() {
    dioxus_logger::init(Args::parse().log_level).expect("Failed to initialize dioxus logger");

    let config = LaunchBuilder::new(App);

    #[cfg(target_arch = "wasm32")]
    {
        log::info!("web launching");
        dioxus_web::launch(App);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use dioxus_desktop::{Config, WindowBuilder};
        server_fn::set_server_url("https://oxide-chess.fly.dev");

        log::info!("desktop launching");

        dioxus_desktop::launch_cfg(
            App,
            Config::new()
                .with_window(
                    WindowBuilder::new()
                        .with_title("Oxide Chess")
                        .with_maximized(true),
                )
                .with_disable_context_menu(true),
        );
    }
}
