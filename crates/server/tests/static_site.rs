use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt as _;
use tower::ServiceExt as _;

/// A stand-in for `web/build`: two prerendered shells, the 404 fallback, a
/// hashed asset, and a precompressed engine file.
fn fake_build() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    std::fs::write(p.join("index.html"), "<h1>play</h1>").unwrap();
    std::fs::write(p.join("analysis.html"), "<h1>analysis</h1>").unwrap();
    std::fs::write(p.join("404.html"), "<h1>shell</h1>").unwrap();
    std::fs::create_dir_all(p.join("_app/immutable/chunks")).unwrap();
    std::fs::write(p.join("_app/immutable/chunks/abc.js"), "console.log(1)").unwrap();
    std::fs::create_dir_all(p.join("engine")).unwrap();
    std::fs::write(p.join("engine/sf.wasm"), b"\0asm").unwrap();
    std::fs::write(p.join("engine/sf.wasm.gz"), b"gzipped").unwrap();
    dir
}

async fn get(
    path: &str,
    accept_encoding: Option<&str>,
) -> (StatusCode, axum::http::HeaderMap, String) {
    let dir = fake_build();
    let app = server::app(dir.path());
    let mut req = Request::builder().uri(path);
    if let Some(enc) = accept_encoding {
        req = req.header(header::ACCEPT_ENCODING, enc);
    }
    let response = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn prerendered_pages_are_served_by_clean_url() {
    let (status, _, body) = get("/", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "<h1>play</h1>");

    let (status, _, body) = get("/analysis", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "<h1>analysis</h1>");
    let (status, _, body) = get("/analysis/?x=1", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "<h1>analysis</h1>");

    // No traversal through the rewrite, and no rewrite for real files.
    let (status, _, _) = get("/../analysis", None).await;
    assert_ne!(status, StatusCode::OK);
    let (status, _, body) = get("/analysis.html", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "<h1>analysis</h1>");
}

#[tokio::test]
async fn unknown_paths_get_the_shell_with_a_404_status() {
    let (status, headers, body) = get("/game/42", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, "<h1>shell</h1>");
    assert!(
        headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
}

#[tokio::test]
async fn every_response_is_cross_origin_isolated() {
    for path in [
        "/",
        "/analysis",
        "/nope",
        "/_app/immutable/chunks/abc.js",
        "/healthz",
    ] {
        let (_, headers, _) = get(path, None).await;
        assert_eq!(
            headers["cross-origin-opener-policy"], "same-origin",
            "{path}"
        );
        assert_eq!(
            headers["cross-origin-embedder-policy"], "require-corp",
            "{path}"
        );
    }
}

#[tokio::test]
async fn hashed_assets_are_immutable_and_pages_are_not() {
    let (status, headers, _) = get("/_app/immutable/chunks/abc.js", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );

    let (_, headers, _) = get("/", None).await;
    assert!(headers.get(header::CACHE_CONTROL).is_none());
    // A miss under the immutable prefix must not be cached forever either.
    let (status, headers, _) = get("/_app/immutable/chunks/gone.js", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(headers.get(header::CACHE_CONTROL).is_none());
}

#[tokio::test]
async fn precompressed_files_are_used_when_accepted() {
    let (status, headers, body) = get("/engine/sf.wasm", Some("gzip, br")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_ENCODING], "gzip");
    assert_eq!(body, "gzipped");

    let (_, headers, body) = get("/engine/sf.wasm", None).await;
    assert!(headers.get(header::CONTENT_ENCODING).is_none());
    assert_eq!(body, "\0asm");
}

#[tokio::test]
async fn healthz() {
    let (status, _, body) = get("/healthz", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}
