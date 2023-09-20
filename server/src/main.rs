use axum::{
    extract::{Path, WebSocketUpgrade},
    routing::get,
    ServiceExt,
};
use common::args::*;
use dioxus_fullstack::prelude::*;
use server::database;
use server::game_socket::handler;
use tower::ServiceExt as OtherServiceExt;
use tower_http::services::ServeFile;

#[tokio::main]
pub async fn main() {
    dioxus_logger::init(Args::parse().log_level).expect("Failed to initialize dioxus logger");

    // // Initialize database connection
    // let db_client = database::connect()
    //     .await
    //     .expect("Failed to connect to database");

    log::info!("server launching");
    let addr = "[::]:8080".parse().unwrap();
    log::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(
            axum::Router::new()
                .nest_service("/", ServeFile::new("dist/index.html"))
                .register_server_fns("/api")
                .route(
                    "/game/:game_id",
                    get(move |Path::<u32>(game_id), ws: WebSocketUpgrade| handler(game_id, ws)),
                )
                .serve_static_assets("dist")
                .map_response(|mut response| {
                    response
                        .headers_mut()
                        .insert("Cross-Origin-Opener-Policy", "same-origin".parse().unwrap());
                    response.headers_mut().insert(
                        "Cross-Origin-Embedder-Policy",
                        "require-corp".parse().unwrap(),
                    );
                    response
                })
                .into_make_service(),
        )
        .await
        .unwrap();
}
