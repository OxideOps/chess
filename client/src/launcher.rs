use dioxus_fullstack::prelude::LaunchBuilder;

pub fn launch() {
    let mut config = LaunchBuilder::new(crate::components::App);
    #[cfg(not(target_arch = "wasm32"))]
    configure_desktop(&mut config);

    #[cfg(target_arch = "wasm32")]
    configure_web(&mut config);

    config.launch();
}

#[cfg(not(target_arch = "wasm32"))]
fn configure_desktop<Props: Clone>(config: &mut LaunchBuilder<Props>) {
    use dioxus_desktop::{Config, WindowBuilder};
}

#[cfg(target_arch = "wasm32")]
fn configure_web<Props: Clone>(builder: &mut config<Props>) {
    
}