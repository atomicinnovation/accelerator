use axum::Router;

/// Relative path from this crate's manifest directory to the built frontend.
/// Single source of truth for the dist-path literal — `build.rs` and the
/// rust-embed `#[folder = ...]` proc-macro attribute duplicate this value
/// because they cannot reference the constant directly (build.rs runs
/// before the crate compiles; proc-macros require string literals). Keep
/// all three in sync.
pub const FRONTEND_DIST_REL: &str = "../frontend/dist";

/// Resolves `FRONTEND_DIST_REL` against this crate's manifest directory.
pub fn default_dist_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FRONTEND_DIST_REL)
}

/// Attach SPA asset serving to `router` using the default dist path.
///
/// All paths not already claimed by API routes fall through to this handler.
/// Unknown paths fall back to `index.html` for client-side routing.
pub fn apply_spa_serving(router: Router) -> Router {
    #[cfg(feature = "dev-frontend")]
    { apply_spa_serving_with_dist_path(router, default_dist_path()) }
    #[cfg(not(feature = "dev-frontend"))]
    { apply_spa_serving_inner(router) }
}

// ── dev-frontend mode ──────────────────────────────────────────────────────

#[cfg(feature = "dev-frontend")]
pub fn apply_spa_serving_with_dist_path(
    router: Router,
    dist_path: std::path::PathBuf,
) -> Router {
    use tower_http::services::{ServeDir, ServeFile};
    // Use `.fallback()` (not `.not_found_service()`). In tower-http 0.5,
    // `not_found_service` wraps the fallback in SetStatus(404) — it forces
    // 404 even when the fallback (ServeFile) would return 200. For SPA
    // client-side routing we need 200, so `.fallback()` is correct here.
    router.fallback_service(
        ServeDir::new(&dist_path)
            .fallback(ServeFile::new(dist_path.join("index.html"))),
    )
}

// ── shared helpers (both modes) ───────────────────────────────────────────

/// Normalise a URI path into an embedded-asset key. Empty / root-only
/// paths map to `"index.html"`; the leading slash is stripped otherwise.
///
/// Intentionally does NOT sanitise traversal sequences — that belongs to
/// rust-embed (compile-time HashMap lookup, no filesystem resolution) and
/// tower-http `ServeDir` (path-traversal hardened).
#[cfg(any(test, not(feature = "dev-frontend")))]
fn normalise_asset_path(uri_path: &str) -> &str {
    let trimmed = uri_path.trim_start_matches('/');
    if trimmed.is_empty() { "index.html" } else { trimmed }
}

// ── embed-dist mode ────────────────────────────────────────────────────────

#[cfg(not(feature = "dev-frontend"))]
#[derive(rust_embed::Embed)]
#[folder = "../frontend/dist"]
struct Frontend;

#[cfg(not(feature = "dev-frontend"))]
fn apply_spa_serving_inner(router: Router) -> Router {
    router.fallback(embedded_fallback)
}

#[cfg(not(feature = "dev-frontend"))]
async fn embedded_fallback(
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    serve_embedded::<Frontend>(uri.path())
}

/// Serve an asset from embed type `E`, falling back to `index.html` for
/// unknown paths. Generic so tests can substitute a fixture embed
/// (`tests/fixtures/mini-dist/`) without pulling in the real frontend.
#[cfg(not(feature = "dev-frontend"))]
fn serve_embedded<E: rust_embed::Embed>(uri_path: &str) -> axum::response::Response {
    use axum::{http::{header, StatusCode}, response::IntoResponse};

    let path = normalise_asset_path(uri_path);

    match E::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mime_str = mime.as_ref();
            let ct = if mime_str.starts_with("text/")
                || mime_str == "application/javascript"
                || mime_str == "application/json"
            {
                format!("{mime_str}; charset=utf-8")
            } else {
                mime_str.to_string()
            };
            ([(header::CONTENT_TYPE, ct)], content.data).into_response()
        }
        None => match E::get("index.html") {
            Some(content) => (
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                content.data,
            )
                .into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        },
    }
}

// ── Pure path-normalisation tests (both feature modes) ────────────────────
#[cfg(test)]
mod path_normalisation_tests {
    use super::normalise_asset_path;

    #[test]
    fn root_maps_to_index_html() {
        assert_eq!(normalise_asset_path("/"), "index.html");
    }

    #[test]
    fn empty_maps_to_index_html() {
        assert_eq!(normalise_asset_path(""), "index.html");
    }

    #[test]
    fn leading_slash_is_stripped_for_asset_paths() {
        assert_eq!(normalise_asset_path("/assets/app.js"), "assets/app.js");
    }

    #[test]
    fn nested_client_routes_are_preserved_as_keys() {
        assert_eq!(
            normalise_asset_path("/library/decisions/0007-foo"),
            "library/decisions/0007-foo",
        );
    }

    #[test]
    fn traversal_segments_passed_through_by_design() {
        // Normalisation does NOT sanitise traversal sequences — that
        // responsibility belongs to rust-embed (HashMap key lookup, no
        // path resolution) and tower-http `ServeDir` (path-traversal
        // hardened). See embed_tests::traversal_path_falls_back_to_index_html
        // and the equivalent ServeDir integration coverage.
        assert_eq!(normalise_asset_path("/../etc/passwd"), "../etc/passwd");
    }
}

// ── Embed-dist handler tests (embed-dist feature only) ────────────────────
#[cfg(all(test, not(feature = "dev-frontend")))]
mod embed_tests {
    use super::serve_embedded;
    use axum::http::{header, StatusCode};
    use http_body_util::BodyExt as _;

    #[derive(rust_embed::Embed)]
    #[folder = "tests/fixtures/mini-dist"]
    struct TestFrontend;

    async fn body_of(resp: axum::response::Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        std::str::from_utf8(&bytes).unwrap().to_string()
    }

    #[tokio::test]
    async fn known_asset_is_served_with_mime_type() {
        let resp = serve_embedded::<TestFrontend>("/assets/app.js");
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(ct.contains("javascript"), "expected js mime, got: {ct}");
    }

    #[tokio::test]
    async fn unknown_path_falls_back_to_index_html() {
        let resp = serve_embedded::<TestFrontend>("/library/decisions");
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_of(resp).await.contains("mini-dist index"));
    }

    #[tokio::test]
    async fn root_path_serves_index_html() {
        let resp = serve_embedded::<TestFrontend>("/");
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_of(resp).await.contains("mini-dist index"));
    }

    #[tokio::test]
    async fn traversal_path_falls_back_to_index_html_no_os_read() {
        // rust-embed is a compile-time HashMap of embedded bytes; there is
        // no filesystem lookup at request time, so `../etc/passwd` cannot
        // escape the embed. The handler falls back to index.html.
        let resp = serve_embedded::<TestFrontend>("/../etc/passwd");
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_of(resp).await.contains("mini-dist index"));
    }
}

// ── Dev-frontend handler tests (dev-frontend feature only) ────────────────
#[cfg(all(test, feature = "dev-frontend"))]
mod dev_frontend_tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    fn make_dist(tmp: &std::path::Path) {
        std::fs::write(tmp.join("index.html"), "<html>spa</html>").unwrap();
        std::fs::create_dir_all(tmp.join("assets")).unwrap();
        std::fs::write(tmp.join("assets/app.js"), "// js").unwrap();
    }

    #[tokio::test]
    async fn known_static_file_is_served() {
        let dist = tempfile::tempdir().unwrap();
        make_dist(dist.path());
        let app = apply_spa_serving_with_dist_path(
            Router::new(),
            dist.path().to_path_buf(),
        );

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/assets/app.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(ct.contains("javascript") || ct.contains("text"), "ct: {ct}");
    }

    #[tokio::test]
    async fn unknown_path_falls_back_to_index_html() {
        let dist = tempfile::tempdir().unwrap();
        make_dist(dist.path());
        let app = apply_spa_serving_with_dist_path(
            Router::new(),
            dist.path().to_path_buf(),
        );

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/library/decisions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert!(std::str::from_utf8(&body).unwrap().contains("spa"));
    }

    #[tokio::test]
    async fn root_path_serves_index_html() {
        let dist = tempfile::tempdir().unwrap();
        make_dist(dist.path());
        let app = apply_spa_serving_with_dist_path(
            Router::new(),
            dist.path().to_path_buf(),
        );

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }
}
