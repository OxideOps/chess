mod arrows;
mod components;
mod game_socket;
mod helpers;
mod mouse_click;
mod router;
pub mod shared_states;
mod stockfish;
#[cfg(feature = "web")]
mod storage;
mod system_info;

pub fn launch() {
    #[cfg(feature = "desktop")]
    {
        use dioxus_desktop as desktop;
        use dioxus_fullstack::prelude::server_fn;

        server_fn::set_server_url("https://oxide-chess.fly.dev/");
        log::info!("configuring desktop..");
        desktop::launch_cfg(
            components::App,
            desktop::Config::new()
                .with_window(
                    desktop::WindowBuilder::new()
                        .with_title("Oxide Chess")
                        .with_maximized(true),
                )
                .with_disable_context_menu(true),
        )
    }

    #[cfg(feature = "web")]
    dioxus_web::launch(components::App);
}
