use axum::extract::Path;
use axum::{extract::WebSocketUpgrade, routing::get};
use common::args::*;
use dioxus_fullstack::prelude::*;
use server::game_socket::{handler, PlayerConnections};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeFile;

#[tokio::main]
pub async fn main() {
    dioxus_logger::init(Args::parse().log_level).expect("Failed to initialize dioxus logger");

    log::info!("server launching");

    let connections: PlayerConnections = Default::default();
    let connected: Arc<RwLock<bool>> = Default::default();
    let addr = "[::]:8080".parse().unwrap();
    log::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(
            axum::Router::new()
                .nest_service("/", ServeFile::new("dist/index.html"))
                .serve_static_assets("dist")
                .register_server_fns("/api")
                .route(
                    "/game/:game_id",
                    get(move |Path::<u32>(game_id), ws: WebSocketUpgrade| {
                        handler(game_id, ws, connections, connected.clone())
                    }),
                )
                .into_make_service(),
        )
        .await
        .unwrap();
}
