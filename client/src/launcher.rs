pub fn launch() {
    #[cfg(not(target_arch= "wasm32"))]
    {
        use dioxus_fullstack::prelude::server_fn;
        use dioxus_desktop::{Config, WindowBuilder};

        log::info!("configuring desktop..");
        server_fn::set_server_url("https://oxide-chess.fly.dev");
    
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
