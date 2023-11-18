use dioxus_fullstack::prelude::*;
pub fn launch() {
    let mut builder = LaunchBuilder::new(crate::components::App);
    #[cfg(feature = "desktop")]
    {
        use dioxus_desktop::{Config, WindowBuilder};
        //server_fn::set_server_url("https://oxide-chess.fly.dev");

        log::info!("configuring desktop..");
        builder = builder.desktop_cfg(
            Config::new()
                .with_window(
                    WindowBuilder::new()
                        .with_title("Oxide Chess")
                        .with_maximized(true),
                )
                .with_disable_context_menu(true),
        );
    }
    builder.launch()
}
