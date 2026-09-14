use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

/// Serve the chess web client (or, with a subcommand, maintain its data).
#[derive(Parser, Debug)]
#[command(name = "chess-server", version)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Directory with the client build (`pnpm build` in web/).
    #[arg(long, env = "CHESS_STATIC_DIR", default_value_os_t = server::default_static_dir())]
    static_dir: PathBuf,

    /// Address to listen on.
    #[arg(long, env = "CHESS_BIND", default_value = "127.0.0.1:8080")]
    bind: SocketAddr,

    /// Postgres connection URL. Without it games live in memory only and
    /// there are no accounts.
    #[arg(long, env = "DATABASE_URL", global = true)]
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

    /// Seconds a disconnected player has to come back before their game is
    /// aborted (nobody had moved yet) or lost by abandonment.
    #[arg(long, env = "CHESS_ABANDON_AFTER_SECS", default_value_t = 60)]
    abandon_after_secs: u64,

    /// Anthropic API key: turns on the coach, which explains positions on
    /// the analysis board in plain language.
    #[arg(long, env = "CHESS_ANTHROPIC_API_KEY", hide_env_values = true)]
    anthropic_api_key: Option<String>,

    /// The Claude model the coach uses.
    #[arg(long, env = "CHESS_COACH_MODEL", default_value = server::coach::DEFAULT_MODEL)]
    coach_model: String,

    /// Explanations per user per hour (answers from the cache are free).
    #[arg(long, env = "CHESS_COACH_PER_HOUR", default_value_t = 30)]
    coach_per_hour: u32,

    /// Turn on an offline stand-in coach that answers from the engine's lines
    /// alone, without any API. For development and the end-to-end tests.
    #[arg(long, env = "CHESS_FAKE_COACH", default_value_t = false)]
    fake_coach: bool,

    /// Enable a built-in fake OAuth provider that signs in as whatever name
    /// you type. For development and the end-to-end tests only.
    #[arg(long, env = "CHESS_FAKE_OAUTH", default_value_t = false)]
    fake_oauth: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Load puzzles from the Lichess puzzle database CSV
    /// (https://database.lichess.org/#puzzles). Decompress it on the way in:
    /// `zstd -dc lichess_db_puzzle.csv.zst | chess-server import-puzzles -`.
    ImportPuzzles {
        /// The CSV file, or `-` for standard input.
        file: PathBuf,
        /// Skip puzzles players rated lower than this (-100 to 100).
        #[arg(long, default_value_t = 90)]
        min_popularity: i32,
        /// Skip puzzles played fewer times than this.
        #[arg(long, default_value_t = 1000)]
        min_plays: i32,
        /// Skip puzzles whose rating is less settled than this deviation.
        #[arg(long, default_value_t = 90)]
        max_deviation: i32,
        /// Keep at most this many puzzles per 100 rating points.
        #[arg(long, default_value_t = 10_000)]
        per_band: usize,
    },
}

/// `chess-server import-puzzles`.
async fn import_puzzles(
    database_url: Option<&str>,
    file: &std::path::Path,
    options: server::puzzles::ImportOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = database_url.ok_or("importing puzzles needs a database: set DATABASE_URL")?;
    let db = server::db::Db::connect(url).await?;
    let reader: Box<dyn std::io::Read> = if file.as_os_str() == "-" {
        Box::new(std::io::stdin().lock())
    } else {
        Box::new(std::io::BufReader::new(std::fs::File::open(file)?))
    };
    let stats = server::puzzles::import(&db, reader, &options, |s| {
        eprintln!("… {} rows read, {} kept", s.read, s.imported);
    })
    .await
    .map_err(|e| e.to_string())?;
    eprintln!(
        "read {} puzzles: imported {}, below the bar {}, band full {}, invalid {}",
        stats.read, stats.imported, stats.filtered, stats.band_full, stats.invalid
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args = Args::parse();
    if let Some(Command::ImportPuzzles {
        file,
        min_popularity,
        min_plays,
        max_deviation,
        per_band,
    }) = &args.command
    {
        let options = server::puzzles::ImportOptions {
            min_popularity: *min_popularity,
            min_plays: *min_plays,
            max_deviation: *max_deviation,
            per_band: *per_band,
        };
        return import_puzzles(args.database_url.as_deref(), file, options).await;
    }

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
        abandon_after: Some(std::time::Duration::from_secs(args.abandon_after_secs)),
        coach: match (&args.anthropic_api_key, args.fake_coach) {
            (Some(key), _) => Some(server::coach::CoachConfig {
                model: args.coach_model.clone(),
                per_hour: args.coach_per_hour,
                ..server::coach::CoachConfig::anthropic(key.clone())
            }),
            (None, true) => {
                tracing::warn!("the coach is the offline stand-in (--fake-coach)");
                Some(server::coach::CoachConfig::fake())
            }
            (None, false) => None,
        },
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
