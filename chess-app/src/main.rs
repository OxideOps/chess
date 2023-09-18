use dioxus_fullstack::prelude::*;

fn main() {
    let builder = LaunchBuilder::new(None);

    #[cfg(target_arch = "wasm32")]
    {
        client::setup_client(builder.clone());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        server::setup_server(builder.clone());
    }

    builder.launch();
}
