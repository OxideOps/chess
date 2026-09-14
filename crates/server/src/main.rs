use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;
use tracing_subscriber::EnvFilter;

/// Serve the chess web client.
#[derive(Parser, Debug)]
#[command(name = "chess-server", version)]
struct Args {
    /// Directory with the client build (`pnpm build` in web/).
    #[arg(long, env = "CHESS_STATIC_DIR", default_value_os_t = server::default_static_dir())]
    static_dir: PathBuf,

    /// Address to listen on.
    #[arg(long, env = "CHESS_BIND", default_value = "127.0.0.1:8080")]
    bind: SocketAddr,

    /// Postgres connection URL. Without it games live in memory only and
    /// there are no accounts.
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Mark the session cookie `Secure`. Turn on when serving over https.
    #[arg(long, env = "CHESS_SECURE_COOKIES", default_value_t = false)]
    secure_cookies: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args = Args::parse();

    let fallback = args.static_dir.join(server::FALLBACK_PAGE);
    if !fallback.is_file() {
        return Err(format!(
            "{} is not a client build (no {}); run `pnpm build` in web/ or pass --static-dir",
            args.static_dir.display(),
            server::FALLBACK_PAGE
        )
        .into());
    }

    let state = match &args.database_url {
        Some(url) => {
            let db = server::db::Db::connect(url).await?;
            tracing::info!("games and accounts are in Postgres");
            server::AppState::with_db(db, args.secure_cookies)
        }
        None => {
            tracing::warn!("no DATABASE_URL: games are kept in memory only, no accounts");
            server::AppState::in_memory()
        }
    };

    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    tracing::info!(
        "serving {} on http://{}",
        args.static_dir.display(),
        args.bind
    );
    axum::serve(listener, server::app_with(&args.static_dir, state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutting down");
        })
        .await?;
    Ok(())
}
