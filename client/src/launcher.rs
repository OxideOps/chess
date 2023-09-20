use dioxus_fullstack::prelude::LaunchBuilder;

pub fn launch() {
    let _builder = LaunchBuilder::new(crate::components::App);


}

#[cfg(not(target_arch = "wasm32"))]
fn launch_desktop<Props: Clone>(builder: &mut LaunchBuilder<Props>) {

}

#[cfg(target_arch = "wasm32")]
fn launch_web<Props: Clone>(builder: &mut LaunchBuilder<Props>) {

}