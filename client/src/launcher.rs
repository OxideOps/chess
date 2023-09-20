use dioxus_fullstack::prelude::*;

pub fn launch() {
    configure().launch()
}

fn configure() -> LaunchBuilder<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        configure_desktop()
    }
    #[cfg(target_arch = "wasm32")]
    {
        configure_web()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn configure_desktop() -> LaunchBuilder<()> {
    use dioxus_desktop::{Config, WindowBuilder};

    log::info!("configuring desktop..");
    server_fn::set_server_url("https://oxide-chess.fly.dev");

    LaunchBuilder::new(crate::components::App).desktop_cfg(
        Config::new()
            .with_window(
                WindowBuilder::new()
                    .with_title("Oxide Chess")
                    .with_maximized(true),
            )
            .with_disable_context_menu(true),
    )
}

#[cfg(target_arch = "wasm32")]
fn configure_web() -> LaunchBuilder<()> {
    log::info!("configuring web..");
    LaunchBuilder::new(crate::components::App)
}
