pub fn launch() {
    #[cfg(feature = "desktop")]
    {
        use dioxus_desktop as desktop;
        use dioxus_fullstack::prelude::server_fn;

        server_fn::set_server_url("https://oxide-chess.fly.dev/");
        log::info!("configuring desktop..");
        desktop::launch_cfg(
            super::components::App,
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
    dioxus_web::launch(super::components::App);

    #[cfg(feature = "server")]
    {
        use axum::{
            extract::{Path, WebSocketUpgrade},
            routing::get,
            ServiceExt,
        };
        use dioxus_fullstack::prelude::*;
        use tower::ServiceExt as OtherServiceExt;
        use tower_http::services::ServeDir;
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                if dotenvy::dotenv().is_err() {
                    log::warn!(".env file not found, continuing without loading")
                }

                log::info!("server launching");
                let addr = "[::]:8080".parse().unwrap();
                log::info!("listening on {}", addr);
                axum::Server::bind(&addr)
                    .serve(
                        axum::Router::new()
                            .nest_service("/", ServeDir::new("dist"))
                            .nest_service("/images", ServeDir::new("images"))
                            .register_server_fns("/api")
                            .route(
                                "/game/:game_id",
                                get(move |Path::<u32>(game_id), ws: WebSocketUpgrade| {
                                    handler(game_id, ws)
                                }),
                            )
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
