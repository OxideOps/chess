pub fn launch() {
    #[cfg(feature = "desktop")]
    {
        use dioxus_desktop::{Config, WindowBuilder};
        // use dioxus_fullstack::prelude::server_fn;

        log::info!("configuring desktop..");
        // server_fn::set_server_url("https://oxide-chess.fly.dev");

        dioxus_desktop::launch_cfg(
            crate::components::App,
            Config::new()
                .with_window(
                    WindowBuilder::new()
                        .with_title("Oxide Chess")
                        .with_maximized(true),
                )
                .with_disable_context_menu(true),
        );
    }

    #[cfg(target_arch = "wasm32")]
    dioxus_web::launch(crate::components::App);
}
