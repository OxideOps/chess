use dioxus_fullstack::prelude::*;

pub fn launch() {
    let mut builder = LaunchBuilder::new(crate::components::App);
    #[cfg(not(target_arch = "wasm32"))]
    configure_desktop(&mut config);

    #[cfg(target_arch = "wasm32")]
    configure_web(&mut config);

    config.launch();
}

#[cfg(not(target_arch = "wasm32"))]
fn configure_desktop<Props: Clone>(builder: &mut LaunchBuilder<Props>) {
    use dioxus_desktop::{Config, WindowBuilder};

    server_fn::set_server_url("https://oxide-chess.fly.dev");
}

#[cfg(target_arch = "wasm32")]
fn configure_web<Props: Clone>(builder: &mut LaunchBuilder<Props>) {

}