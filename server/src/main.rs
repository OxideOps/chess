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
                .route(
                    "/game/:game_id",
                    get(move |ws: WebSocketUpgrade| handler(ws, connections, connected.clone())),
                )
                .into_make_service(),
        )
        .await
        .unwrap();
}
