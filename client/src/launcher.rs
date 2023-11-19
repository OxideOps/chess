pub fn launch() {
    #[cfg(feature = "desktop")]
    {
        use dioxus_desktop::{Config, WindowBuilder};
        dioxus_fullstack::prelude::server_fn::set_server_url("https://oxide-chess.fly.dev");
        log::info!("configuring desktop..");
        dioxus_desktop::launch_cfg(
            super::components::App,
            Config::new()
                .with_window(
                    WindowBuilder::new()
                        .with_title("Oxide Chess")
                        .with_maximized(true),
                )
                .with_disable_context_menu(true),
        )
    }

    #[cfg(feature = "web")]
    dioxus_web::launch(super::components::App);

    #[cfg(feature = "server")]
    {
        use axum::{
            extract::{Path, WebSocketUpgrade},
            routing::get,
            ServiceExt,
        };
        use common::args::*;
        use dioxus_fullstack::prelude::*;
        use tower::ServiceExt as OtherServiceExt;
        use tower_http::services::ServeDir;
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                log::info!("server launching");
                let addr = "[::]:8080".parse().unwrap();
                log::info!("listening on {}", addr);
                axum::Server::bind(&addr)
                    .serve(
                        axum::Router::new()
                            .nest_service("/", ServeDir::new("dist"))
                            .nest_service("/images", ServeDir::new("images"))
                            .map_response(|mut response| {
                                response.headers_mut().insert(
                                    "Cross-Origin-Opener-Policy",
                                    "same-origin".parse().unwrap(),
                                );
                                response.headers_mut().insert(
                                    "Cross-Origin-Embedder-Policy",
                                    "require-corp".parse().unwrap(),
                                );
                                response
                            })
                            .into_make_service(),
                    )
                    .await
                    .unwrap()
            });
    }
}
