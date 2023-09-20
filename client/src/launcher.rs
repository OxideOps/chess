use dioxus_fullstack::prelude::*;

pub fn launch() {
    let mut builder = LaunchBuilder::new(crate::components::App);

    #[cfg(not(target_arch = "wasm32"))]
    {
        builder = configure_desktop(builder);
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        builder = configure_web(builder);
    }

    builder.launch();
}

#[cfg(not(target_arch = "wasm32"))]
fn configure_desktop(builder: LaunchBuilder<()>) -> LaunchBuilder<()>{
    use dioxus_desktop::{Config, WindowBuilder};
    
    log::info!("configuring desktop..");
    server_fn::set_server_url("https://oxide-chess.fly.dev");
    builder.desktop_cfg(
        Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("Oxide Chess")
                .with_maximized(true),
        )
        .with_disable_context_menu(true)
    )
}

#[cfg(target_arch = "wasm32")]
fn configure_web<Props: Clone>(mut builder: LaunchBuilder<()>) -> LaunchBuilder<()> {
    log::info!("configuring web..");
    builder
}