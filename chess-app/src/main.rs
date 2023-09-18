use chess_app::*;
use dioxus_fullstack::prelude::*;
fn main() {
    let mut config = LaunchBuilder::new(App);

    core::setup_config(&mut config);

    config.launch()
}
