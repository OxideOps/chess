use chess_app::*;
use dioxus_fullstack::prelude::*;
fn main() {
    let mut config = LaunchBuilder::new(App);

    #[cfg(feature = "client")]
    client::setup_config(&mut config);

    #[cfg(feature = "server")]
    server::setup_config(&mut config);

    config.launch()
}
