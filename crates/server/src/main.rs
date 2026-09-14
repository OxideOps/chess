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

    /// Take client addresses from `X-Forwarded-For`. Turn on only behind a
    /// reverse proxy that sets it (rate limits are per address).
    #[arg(long, env = "CHESS_TRUST_PROXY", default_value_t = false)]
    trust_proxy: bool,

    /// Extra origins allowed to open game sockets, comma-separated
    /// (e.g. the public URL when a proxy rewrites `Host`).
    #[arg(long, env = "CHESS_ALLOWED_ORIGINS", value_delimiter = ',')]
    allowed_origins: Vec<String>,

    /// Where browsers reach this server, e.g. `https://chess.example`; used
    /// for OAuth redirect URLs. Defaults to the request's own host.
    #[arg(long, env = "CHESS_PUBLIC_URL")]
    public_url: Option<String>,

    /// Enable "Sign in with Lichess". Any name identifies the app; Lichess
    /// needs no registration or secret.
    #[arg(long, env = "CHESS_LICHESS_CLIENT_ID")]
    lichess_client_id: Option<String>,

    /// Enable "Sign in with Google" (a Google Cloud OAuth client with the
    /// `/api/auth/google/callback` redirect URL registered).
    #[arg(
        long,
        env = "CHESS_GOOGLE_CLIENT_ID",
        requires = "google_client_secret"
    )]
    google_client_id: Option<String>,

    #[arg(long, env = "CHESS_GOOGLE_CLIENT_SECRET", hide_env_values = true)]
    google_client_secret: Option<String>,

    /// Enable a built-in fake OAuth provider that signs in as whatever name
    /// you type. For development and the end-to-end tests only.
    #[arg(long, env = "CHESS_FAKE_OAUTH", default_value_t = false)]
    fake_oauth: bool,
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

    let mut oauth = Vec::new();
    if let Some(id) = &args.lichess_client_id {
        oauth.push(server::oauth::Provider::lichess(id.clone()));
    }
    if let (Some(id), Some(secret)) = (&args.google_client_id, &args.google_client_secret) {
        oauth.push(server::oauth::Provider::google(id.clone(), secret.clone()));
    }
    if args.fake_oauth {
        tracing::warn!("the fake OAuth provider is on: anyone can sign in as any name");
        oauth.push(server::oauth::Provider::fake());
    }
    let config = server::Config {
        secure_cookies: args.secure_cookies,
        trust_proxy: args.trust_proxy,
        allowed_origins: args.allowed_origins.clone(),
        public_url: args.public_url.clone(),
        oauth,
    };
    let state = match &args.database_url {
        Some(url) => {
            let db = server::db::Db::connect(url).await?;
            tracing::info!("games and accounts are in Postgres");
            let state = server::AppState::with_db(db, config);
            // Expired sessions and abandoned guests are swept in the background.
            server::auth::spawn_cleanup(state.auth.clone().expect("with_db sets auth"));
            state
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
    // `ConnectInfo` gives the handlers the client address for rate limiting.
    let app = server::app_with(&args.static_dir, state)
        .into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutting down");
        })
        .await?;
    Ok(())
}
