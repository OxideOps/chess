use chess::app::App;
use clap::Parser;

/// Chess program
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// One of TRACE, DEBUG, INFO, WARN, or ERROR
    #[arg(short, long, default_value = "DEBUG")]
    log_level: log::LevelFilter,
}

pub fn main() {
    dioxus_logger::init(Args::parse().log_level).expect("Failed to initialize dioxus logger");

    #[cfg(feature = "web")]
    {
        log::info!("web launching");
        dioxus_web::launch_cfg(App, dioxus_web::Config::new().hydrate(true));
    }
    #[cfg(feature = "server")]
    {
        log::info!("server launching");
        use axum::{extract::WebSocketUpgrade, routing::get};
        use chess::server::game_socket::{handler, PlayerConnections};
        use dioxus_fullstack::prelude::*;
        use std::sync::Arc;
        use tokio::sync::RwLock;
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let connections: PlayerConnections = Default::default();
                let connected: Arc<RwLock<bool>> = Default::default();
                let addr = "[::]:8080".parse().unwrap();
                println!("listening on {}", addr);
                axum::Server::bind(&addr)
                    .serve(
                        axum::Router::new()
                            .route(
                                "/:game_id",
                                get(move |ws: WebSocketUpgrade| {
                                    handler(ws, connections, connected.clone())
                                }),
                            )
                            .serve_dioxus_application("", ServeConfigBuilder::new(App, ()))
                            .into_make_service(),
                    )
                    .await
                    .unwrap();
            });
    }
    #[cfg(feature = "desktop")]
    {
        log::info!("desktop launching");
        use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
        const WINDOW_SIZE: u32 = 800;
        dioxus_desktop::launch_cfg(
            App,
            Config::new().with_window(WindowBuilder::new().with_title("Chess").with_inner_size(
                LogicalSize {
                    width: WINDOW_SIZE,
                    height: WINDOW_SIZE,
                },
            )),
        );
    }
}
