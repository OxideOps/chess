//! The chess server.
//!
//! Serves the static SvelteKit build (with the headers the browser engine
//! needs and a proper 404 fallback) and hosts games: [`games`] has the
//! endpoints, [`room`] the rules of a game as the server enforces them.
//! Games live in memory until persistence arrives.

pub mod auth;
pub mod db;
pub mod games;
pub mod limit;
pub mod origin;
pub mod room;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    Router,
    extract::{FromRef, Request, State},
    http::{HeaderName, HeaderValue, StatusCode, Uri, header},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

/// The name of the file the client build writes for unknown routes.
pub const FALLBACK_PAGE: &str = "404.html";

/// Deployment settings the handlers consult.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Mark the session cookie `Secure`. Set when serving over https.
    pub secure_cookies: bool,
    /// Take the client address from `X-Forwarded-For` (rightmost entry).
    /// Only behind a reverse proxy that sets it; otherwise clients could
    /// pick their own address and sidestep the rate limits.
    pub trust_proxy: bool,
    /// Origins allowed to open game sockets besides the request's own
    /// host, e.g. the public URL when a proxy rewrites `Host`.
    pub allowed_origins: Vec<String>,
}

/// Everything the handlers share.
#[derive(Clone)]
pub struct AppState {
    pub games: games::Games,
    /// `None` when running without a database: no accounts.
    pub auth: Option<auth::Auth>,
    pub config: Arc<Config>,
}

impl AppState {
    /// In-memory games, no accounts.
    pub fn in_memory() -> AppState {
        AppState {
            games: games::Games::default(),
            auth: None,
            config: Arc::default(),
        }
    }

    /// Games and accounts on a database.
    pub fn with_db(db: db::Db, config: Config) -> AppState {
        AppState {
            games: games::Games::with_db(db.clone()),
            auth: Some(auth::Auth::new(db, config.secure_cookies)),
            config: Arc::new(config),
        }
    }
}

impl FromRef<AppState> for games::Games {
    fn from_ref(state: &AppState) -> Self {
        state.games.clone()
    }
}

/// Build the router: the game API plus the static client from `static_dir`
/// (the output of `pnpm build`).
///
/// - Files are served as-is, with precompressed `.br`/`.gz` siblings used
///   when the client accepts them. `/analysis` serves `analysis.html`: the
///   build writes one prerendered shell per route with clean URLs.
/// - Anything else gets `404.html` with a 404 status: the SPA shell renders
///   its own "not found" page, and dynamic routes added later still work.
/// - Every response carries `Cross-Origin-Opener-Policy: same-origin` and
///   `Cross-Origin-Embedder-Policy: require-corp`, which is what enables
///   `SharedArrayBuffer` (and so multi-threaded engines) in browsers.
/// - `/_app/immutable/*` is content-hashed by the build and cached forever.
pub fn app(static_dir: impl AsRef<Path>) -> Router {
    app_with(static_dir, AppState::in_memory())
}

/// [`app`] with explicit state (tests share it across requests).
pub fn app_with(static_dir: impl AsRef<Path>, state: AppState) -> Router {
    let static_dir = static_dir.as_ref().to_path_buf();
    let fallback = ServeFile::new(static_dir.join(FALLBACK_PAGE));
    let files = ServeDir::new(&static_dir)
        .precompressed_br()
        .precompressed_gzip()
        .not_found_service(fallback);

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .merge(games::router())
        .merge(auth::router())
        .with_state(state)
        .fallback_service(files)
        .layer(middleware::from_fn_with_state(
            static_dir,
            serve_prerendered_pages,
        ))
        .layer(middleware::from_fn(cache_immutable_assets))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("cross-origin-opener-policy"),
            HeaderValue::from_static("same-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("cross-origin-embedder-policy"),
            HeaderValue::from_static("require-corp"),
        ))
        .layer(TraceLayer::new_for_http())
}

/// Where the client build lives by default, relative to the working directory.
pub fn default_static_dir() -> PathBuf {
    PathBuf::from("web/build")
}

/// Rewrite `/analysis` to `/analysis.html` when the build has that page, so
/// prerendered routes work with the clean URLs the app links to.
async fn serve_prerendered_pages(
    State(static_dir): State<PathBuf>,
    mut request: Request,
    next: Next,
) -> Response {
    if let Some(page) = prerendered_page(&static_dir, request.uri()) {
        let query = request
            .uri()
            .query()
            .map(|q| format!("?{q}"))
            .unwrap_or_default();
        if let Ok(uri) = format!("{page}{query}").parse::<Uri>() {
            *request.uri_mut() = uri;
        }
    }
    next.run(request).await
}

/// `Some("/analysis.html")` if `uri` is a clean route path with a matching
/// prerendered file; `None` for `/`, for paths with an extension, for
/// anything that isn't a plain relative path, and for misses.
fn prerendered_page(static_dir: &Path, uri: &Uri) -> Option<String> {
    let path = uri.path().trim_end_matches('/');
    let relative = path.strip_prefix('/')?;
    if relative.is_empty() || relative.rsplit('/').next()?.contains('.') {
        return None;
    }
    let safe = Path::new(relative)
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)));
    let file = format!("{relative}.html");
    (safe && static_dir.join(&file).is_file()).then(|| format!("/{file}"))
}

async fn cache_immutable_assets(request: Request, next: Next) -> Response {
    let immutable = request.uri().path().starts_with("/_app/immutable/");
    let mut response = next.run(request).await;
    if immutable && response.status() == StatusCode::OK {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    response
}
