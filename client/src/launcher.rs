use dioxus_fullstack::prelude::LaunchBuilder;

pub fn launch() {
    let config = LaunchBuilder::new(crate::components::App);


}

#[cfg(not(target_arch = "wasm32"))]
fn configure_desktop<Props: Clone>(builder: &mut LaunchBuilder<Props>) {

}

#[cfg(target_arch = "wasm32")]
fn configure_web<Props: Clone>(builder: &mut LaunchBuilder<Props>) {

}