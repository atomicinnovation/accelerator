---
date: "2026-04-22T00:00:00+01:00"
type: plan
skill: create-plan
status: draft
---

# Meta visualiser Phase 5 — Frontend scaffold and library view

## Overview

Phase 5 adds the React SPA. By the end of this phase the browser at
`http://localhost:<port>` shows a real UI: a sidebar listing all 10 doc types,
a library index table for each type, a doc-detail page with rendered markdown and
frontmatter chips, and a full three-tier templates view. Lifecycle and Kanban
routes exist as stubs so the sidebar is complete from day one.

The approach is test-driven throughout. Server-side changes (Cargo features,
`build.rs`, `assets.rs`) get Rust unit/integration tests written first. Frontend
utilities and components get Vitest + React Testing Library tests written first
or alongside implementation.

## Current state

Phases 1–4 are complete. The server has:

- All read-only API endpoints: `/api/types`, `/api/docs`, `/api/docs/*path`,
  `/api/templates`, `/api/templates/:name`, `/api/lifecycle`,
  `/api/lifecycle/:slug`, `/api/events` (SSE).
- `AppState` with `indexer`, `templates`, `clusters`, `sse_hub`, `activity`.
- `GET /` returns a placeholder string — no HTML served.
- No `frontend/` directory exists; `.gitignore` already excludes `frontend/dist/`
  and `frontend/node_modules/`.
- `node` is absent from `mise.toml`; `rust-embed` and `mime_guess` are absent
  from `Cargo.toml`.

## Desired end state

- `cargo run --features dev-frontend` serves the SPA from `frontend/dist/`.
- `cargo build` (default `embed-dist` feature) embeds `frontend/dist/` into the
  binary — requires `npm run build` to have been run first.
- Opening `http://localhost:<port>` loads the React app.
- The sidebar lists all 10 doc types grouped correctly; Templates is de-emphasised
  under a "Meta" heading; Lifecycle and Kanban nav items are present as stubs.
- `/library/:type` shows a sortable table of all docs of that type.
- `/library/:type/:fileSlug` shows the doc with rendered markdown (CommonMark +
  GFM + syntax highlighting) and frontmatter chips.
- `/library/templates/:name` shows the three-tier panel layout (plugin default ·
  user override · config override) with presence indicators and an active badge.
- Live-update: editing a file on disk triggers an SSE event which invalidates the
  relevant TanStack Query caches; the UI re-fetches silently.
- `cargo test --features dev-frontend` and `npm run test` both pass.

### Verification

```bash
# Server unit tests (includes assets.rs tests under dev-frontend feature):
cd skills/visualisation/visualise/server
cargo test --lib --features dev-frontend

# Server integration tests:
cargo test --tests --features dev-frontend

# Frontend unit/component tests:
cd ../frontend
npm run test

# Full build smoke (requires both npm build and cargo build):
npm run build
cd ../server
cargo build 2>&1 | grep -c error   # must output 0

# Manual: start dev server and verify in browser
ACCELERATOR_VISUALISER_BIN=$(pwd)/target/debug/accelerator-visualiser \
  ../scripts/launch-server.sh
```

## What we are NOT doing

- Lifecycle view (Phase 6).
- Kanban (Phases 7–8) — the kanban route is a stub only.
- `PATCH /api/docs/…/frontmatter` write path (Phase 8).
- `dnd-kit` drag-and-drop (Phase 7).
- Wiki-link resolution in markdown (Phase 9).
- Mermaid rendering (Phase 11 Playwright smoke test).
- "Related artifacts" aside (empty component only in Phase 5; wired in Phase 9).
- Playwright E2E tests (Phase 11).
- Watching template tier-1 and tier-2 directories (deferred per Phase 4 plan).
- Error handling polish, JSON logging, SSE reconnect backoff (Phase 10).

---

## Step 1: Cargo features + `build.rs` (TDD)

### 1a. Update `Cargo.toml`

**File**: `skills/visualisation/visualise/server/Cargo.toml`

Add two features and three new dependencies:

```toml
[features]
default = ["embed-dist"]

# embed-dist (production default): bundle frontend/dist/ into the binary
# via rust-embed. Build fails (see build.rs) if frontend/dist/index.html
# is missing. Produces a single-file deployable with no external asset
# dependencies.
embed-dist = ["dep:rust-embed", "dep:mime_guess"]

# dev-frontend (opt-in): serve frontend/dist/ from disk at runtime via
# tower-http ServeDir. Avoids the rust-embed compile-time folder
# requirement, enabling fast Rust iteration without rebuilding the
# frontend. Pulls in tower-http's `fs` feature.
dev-frontend = ["tower-http/fs"]

[dependencies]
# ... (all existing deps unchanged) ...
# `compression` stores embedded assets in brotli-compressed form in the
# binary (reducing binary size). rust-embed decompresses on `get()`, so
# WIRE compression (Content-Encoding: br/gzip) is handled by tower-http's
# CompressionLayer below — not by rust-embed. Keep both for D10 to hold
# (small binary AND small wire payload).
rust-embed = { version = "8", features = ["compression"], optional = true }
mime_guess = { version = "2", optional = true }

# tower-http features:
#   - trace / limit / timeout: existing middleware layers
#   - compression-br / compression-gzip: CompressionLayer wraps responses
#     and emits Content-Encoding based on the request's Accept-Encoding
#     (see build_router in server.rs). Required to meet D10's wire-size
#     target for the SPA assets.
# Note: `fs` (for ServeDir/ServeFile) is added by the `dev-frontend`
# feature above rather than unconditionally, so release builds don't
# carry the filesystem-asset code path.
tower-http = { version = "0.5", features = [
    "trace", "limit", "timeout",
    "compression-br", "compression-gzip",
] }
```

**D10 compression architecture** — two layers with distinct roles:

- `rust-embed`'s `compression` feature → smaller binary. Decompressed on
  every `get()` call, so response bodies handed to axum are already the
  raw asset bytes.
- `tower_http::CompressionLayer` → smaller wire payload. Inspects the
  request's `Accept-Encoding`, compresses the response body accordingly,
  and emits `Content-Encoding: br` / `gzip`.

Neither layer alone is sufficient for D10; together they match a static
server's wire size with a single-binary deployment.

### 1b. Make `DocType.virtual` always serialise

**File**: `skills/visualisation/visualise/server/src/docs.rs`

Phase 3 left `virtual` with `#[serde(default, skip_serializing_if = "std::ops::Not::not")]`,
so the JSON omits the field when `false`. The frontend sidebar needs to
distinguish main doc types from virtual ones via a required boolean, so
we always emit the field now:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocType {
    pub key: DocTypeKey,
    pub label: String,
    pub dir_path: Option<PathBuf>,
    pub in_lifecycle: bool,
    pub in_kanban: bool,
    pub r#virtual: bool,  // always serialised; removed `skip_serializing_if`
}
```

Update any existing unit tests for `describe_types` that asserted the
JSON shape to expect `"virtual": false` on non-template entries.

### 1c. Implement `build.rs`

**File**: `skills/visualisation/visualise/server/build.rs` (new)

```rust
// This value duplicates `accelerator_visualiser::assets::FRONTEND_DIST_REL`.
// `build.rs` runs before the crate compiles, so we cannot import from it.
// Keep the two literals in sync — tests verify the dist path resolves to a
// real directory under the manifest root.
const FRONTEND_DIST_REL: &str = "../frontend/dist";

fn main() {
    let is_embed = std::env::var("CARGO_FEATURE_EMBED_DIST").is_ok();
    if is_embed {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let dist_dir = std::path::Path::new(&manifest).join(FRONTEND_DIST_REL);
        let dist_index = dist_dir.join("index.html");
        if !dist_index.exists() {
            panic!(
                "frontend/dist/index.html not found — \
                 run `npm run build` in skills/visualisation/visualise/frontend/ \
                 before `cargo build` (or use `--features dev-frontend` to \
                 skip embedding and serve from disk instead)"
            );
        }
        println!("cargo:rerun-if-changed={FRONTEND_DIST_REL}");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
```

### Success criteria

```bash
cd skills/visualisation/visualise/server
# Build with dev-frontend (skips embed guard):
cargo build --features dev-frontend 2>&1 | grep -c error   # 0
# Build without frontend/dist/ present and without dev-frontend should fail:
# (don't run this destructively; it's verified by the CI process after npm build)
cargo test --lib --features dev-frontend
```

---

## Step 2: `assets.rs` + wire into `server.rs` (TDD)

### 2a. Write the tests first

Three test modules cover the three layers of asset-serving logic:

1. `path_normalisation_tests` (both features) — pure function `normalise_asset_path`
2. `embed_tests` (embed-dist only) — `serve_embedded<E>` against a checked-in fixture embed
3. `dev_frontend_tests` (dev-frontend only) — `apply_spa_serving_with_dist_path` against a tempdir

Also create the checked-in fixture for the embed test:

**Files**: `skills/visualisation/visualise/server/tests/fixtures/mini-dist/`
  - `index.html` — `<!doctype html><html><body>mini-dist index</body></html>`
  - `assets/app.js` — `// mini-dist stub`

**File**: `skills/visualisation/visualise/server/src/assets.rs` — write the
`#[cfg(test)]` blocks before the implementation:

```rust
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
    use axum::{http::{header, StatusCode}};
    use http_body_util::BodyExt as _;

    // Test-only fixture embed pointing at `tests/fixtures/mini-dist/`.
    // Path is relative to CARGO_MANIFEST_DIR per rust-embed conventions.
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
```

### 2b. Implement `assets.rs`

**File**: `skills/visualisation/visualise/server/src/assets.rs` (new)

```rust
//! SPA asset serving.
//!
//! Two compile-time modes:
//!   - `embed-dist` (default): rust-embed bundles frontend/dist/ into the binary.
//!   - `dev-frontend` (opt-in): tower-http ServeDir reads from disk at runtime.
//!
//! Both modes attach a `.fallback`/`.fallback_service` to the supplied
//! router. The single public entry point for production is
//! `apply_spa_serving`; tests call `apply_spa_serving_with_dist_path` with
//! a seeded tempdir so they exercise the same function production uses.

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
///
/// Under `dev-frontend`, tests should prefer
/// `apply_spa_serving_with_dist_path(router, tmp)` to control the dist
/// source. That helper does not exist under `embed-dist` — the dist is
/// baked into the binary and cannot be swapped at runtime — which keeps
/// the leaky "argument silently ignored" case from arising.
pub fn apply_spa_serving(router: Router) -> Router {
    #[cfg(feature = "dev-frontend")]
    { apply_spa_serving_with_dist_path(router, default_dist_path()) }
    #[cfg(not(feature = "dev-frontend"))]
    { apply_spa_serving_inner(router) }
}

// ── dev-frontend mode ──────────────────────────────────────────────────────
/// Dev-frontend implementation: reads assets from `dist_path` at runtime via
/// tower-http `ServeDir`, with `ServeFile(index.html)` as the not-found
/// fallback for client-side routing.
///
/// Only exists under the `dev-frontend` feature. Callers that need to
/// test the production embed-dist path use `serve_embedded<E>` against
/// a fixture embed type instead.
#[cfg(feature = "dev-frontend")]
pub fn apply_spa_serving_with_dist_path(
    router: Router,
    dist_path: std::path::PathBuf,
) -> Router {
    use tower_http::services::{ServeDir, ServeFile};
    router.fallback_service(
        ServeDir::new(&dist_path)
            .not_found_service(ServeFile::new(dist_path.join("index.html"))),
    )
}

// ── shared helpers (both modes) ───────────────────────────────────────────
/// Normalise a URI path into an embedded-asset key. Empty / root-only
/// paths map to `"index.html"`; the leading slash is stripped otherwise.
///
/// Intentionally does NOT sanitise traversal sequences — that belongs to
/// rust-embed (compile-time HashMap lookup, no filesystem resolution) and
/// tower-http `ServeDir` (path-traversal hardened). Tests under
/// `path_normalisation_tests` pin this behaviour.
fn normalise_asset_path(uri_path: &str) -> &str {
    let trimmed = uri_path.trim_start_matches('/');
    if trimmed.is_empty() { "index.html" } else { trimmed }
}

// ── embed-dist mode ────────────────────────────────────────────────────────
/// The production embedded frontend. Declared at module scope so it is
/// reusable (e.g. by a future bundle-size metric) and so `serve_embedded`
/// can be generic over the embed type.
///
/// Keep this literal in sync with `FRONTEND_DIST_REL` above. Proc-macro
/// attributes require string literals, so this cannot reference the
/// constant directly.
#[cfg(not(feature = "dev-frontend"))]
#[derive(rust_embed::Embed)]
#[folder = "../frontend/dist"]
struct Frontend;

/// Embed-dist implementation of `apply_spa_serving`. Serves from the
/// baked-in rust-embed folder — there is no dist path at runtime, so
/// no `_with_dist_path` variant exists under this feature. Tests that
/// want to exercise the embed-dist handler call `serve_embedded<E>`
/// directly with a fixture embed type.
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
            // Append charset=utf-8 for textual responses (HTML, JS, CSS,
            // JSON) as defence-in-depth against UTF-7 XSS sniffing on
            // older browsers. Binary types (images, fonts) keep their
            // canonical mime with no charset.
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

// tests block shown in §2a above
```

### 2c. Add integration test for SPA serving

**File**: `skills/visualisation/visualise/server/tests/spa_serving.rs` (new)

```rust
//! Integration test: the full router serves the SPA for non-API paths.
//! Only compiled under the dev-frontend feature (dist/ read from disk).

#[cfg(feature = "dev-frontend")]
mod tests {
    use accelerator_visualiser::server::AppState;
    use axum::{body::Body, http::{Request, StatusCode}};
    use http_body_util::BodyExt as _;
    use std::collections::HashMap;
    use tower::ServiceExt as _;

    async fn minimal_state(tmp: &std::path::Path) -> std::sync::Arc<AppState> {
        let cfg = accelerator_visualiser::config::Config {
            plugin_root: tmp.to_path_buf(),
            plugin_version: "test".into(),
            project_root: tmp.to_path_buf(),
            tmp_path: tmp.to_path_buf(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: tmp.join("server.log"),
            doc_paths: HashMap::new(),
            templates: HashMap::new(),
        };
        let activity = std::sync::Arc::new(accelerator_visualiser::activity::Activity::new());
        AppState::build(cfg, activity).await.unwrap()
    }

    // End-to-end integration test: the full `build_router` composition
    // serves the SPA for non-API paths. Uses `build_router_with_dist` to
    // point the SPA fallback at a seeded tempdir, so this runs
    // unconditionally — no dependency on `npm run build` having populated
    // `frontend/dist/`.
    #[tokio::test]
    async fn spa_route_returns_html() {
        let state_tmp = tempfile::tempdir().unwrap();
        let state = minimal_state(state_tmp.path()).await;

        let dist = tempfile::tempdir().unwrap();
        std::fs::write(
            dist.path().join("index.html"),
            "<!doctype html><html>app</html>",
        ).unwrap();

        let app = accelerator_visualiser::server::build_router_with_dist(
            state,
            dist.path().to_path_buf(),
        );

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/library")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(text.contains("<!doctype html") || text.contains("<!DOCTYPE html"),
            "expected HTML, got: {text:.200}");
    }
}
```

### 2d. Wire `assets.rs` into `server.rs`

**File**: `skills/visualisation/visualise/server/src/server.rs`

Replace the `placeholder_root` route and update `build_router`:

```rust
/// Production router. Uses the embedded frontend under `embed-dist`,
/// or the on-disk frontend at `assets::default_dist_path()` under
/// `dev-frontend`. See `build_router_with_dist` for the test variant.
pub fn build_router(state: Arc<AppState>) -> Router {
    build_router_with_spa(state, |router| crate::assets::apply_spa_serving(router))
}

/// Like `build_router` but points the SPA fallback at a caller-supplied
/// `dist_path`. Only exists under `dev-frontend` — under `embed-dist`
/// the dist is baked into the binary and cannot be swapped at runtime,
/// so there is no meaningful `_with_dist_path` variant. Callers that
/// need to test the embed-dist handler use `serve_embedded<E>` with a
/// fixture embed type instead.
#[cfg(feature = "dev-frontend")]
pub fn build_router_with_dist(
    state: Arc<AppState>,
    dist_path: std::path::PathBuf,
) -> Router {
    build_router_with_spa(state, move |router| {
        crate::assets::apply_spa_serving_with_dist_path(router, dist_path)
    })
}

/// Shared composition. The asset-serving function is injected so
/// `build_router` and `build_router_with_dist` don't duplicate the
/// router topology or middleware stack.
fn build_router_with_spa<F: FnOnce(Router) -> Router>(
    state: Arc<AppState>,
    attach_spa: F,
) -> Router {
    // Router topology:
    //   /api/healthz, /api/types, /api/docs/..., /api/templates/..., etc.
    //     → real handlers in `api_router`
    //   /api/*rest  (any other /api path that did not match above)
    //     → `api_not_found` — returns JSON 404. Prevents the SPA fallback
    //       from swallowing /api typos as 200 HTML and masking real API
    //       errors from clients and log auditors.
    //   everything else
    //     → SPA fallback (index.html for client-side routing) via
    //       the `attach_spa` closure.
    //
    // Middleware stack is applied AFTER `attach_spa` so activity
    // tracking, host-header guard, timeout, and body limit wrap both
    // the API routes and the SPA fallback service. SPA asset/HTML
    // fetches count as activity — live browser navigation keeps idle
    // shutdown quiet. Tests lock both invariants in (see below).
    let api_router = Router::new()
        .route("/api/healthz", get(healthz))
        .merge(crate::api::mount(state.clone()))
        // axum 0.7 catch-all syntax — matches `/api/anything-else` that
        // wasn't claimed by a more-specific route above. Curly-brace
        // capture (`{*rest}`) is axum 0.8+ and would not compile here.
        .route("/api/*rest", any(api_not_found))
        .with_state(state.clone());

    attach_spa(api_router)
        // CompressionLayer negotiates Content-Encoding with the client via
        // Accept-Encoding and wraps the response body in a streaming brotli
        // or gzip encoder. Meets D10's wire-size target for the SPA assets;
        // see Cargo.toml for the feature rationale.
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(axum::middleware::from_fn_with_state(
            state.activity.clone(),
            crate::activity::middleware,
        ))
        .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
        .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
        .layer(middleware::from_fn(host_header_guard))
}

/// 404 handler for unmatched `/api/*` paths. Returns JSON rather than
/// letting the SPA fallback serve `index.html` with 200 OK.
async fn api_not_found(
    uri: axum::http::Uri,
) -> impl axum::response::IntoResponse {
    use axum::{http::StatusCode, Json};
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "not-found",
            "path": uri.path(),
        })),
    )
}
```

Remove `placeholder_root` and its route. The existing `serves_placeholder_root_and_writes_info`
test in `server.rs` must be updated: change the body assertion to check for HTML instead of
the old placeholder string (the test must build `frontend/dist/` to pass under `embed-dist`,
so run it under `dev-frontend` feature in the test suite).

**Lock the router-topology and middleware-coverage invariants with tests**
— add the following to `server.rs` (under `#[cfg(feature = "dev-frontend")]`).
The first two assert that the host-header guard and activity middleware
both wrap SPA fallback requests; the third asserts that unmatched `/api/*`
paths return JSON 404 instead of SPA HTML:

```rust
/// Seeds a tempdir with a minimal SPA dist (just index.html) for tests
/// that want to exercise the full `build_router` composition without
/// depending on `frontend/dist/` having been built by npm.
fn seed_stub_dist(tmp: &std::path::Path) {
    std::fs::write(tmp.join("index.html"), "<!doctype html><html>stub</html>").unwrap();
}

#[cfg(feature = "dev-frontend")]
#[tokio::test]
async fn spa_fallback_is_covered_by_host_header_guard() {
    // A request to a non-API path with a non-loopback Host must be rejected.
    let state = /* ... build minimal AppState (see spa_serving.rs pattern) */;
    let dist = tempfile::tempdir().unwrap();
    seed_stub_dist(dist.path());
    let app = build_router_with_dist(state, dist.path().to_path_buf());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/library/decisions")
                .header("host", "evil.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[cfg(feature = "dev-frontend")]
#[tokio::test]
async fn spa_fallback_updates_activity() {
    // A GET to a SPA path must bump Activity::last_millis. Live browser
    // navigation keeps the idle-shutdown watch quiet.
    let state = /* ... build minimal AppState ... */;
    let dist = tempfile::tempdir().unwrap();
    seed_stub_dist(dist.path());
    let before = state.activity.last_millis();
    // Sleep briefly so the atomic has a chance to move forward.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let app = build_router_with_dist(state.clone(), dist.path().to_path_buf());
    let _ = app
        .oneshot(
            Request::builder()
                .uri("/library/decisions")
                .header("host", "127.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let after = state.activity.last_millis();
    assert!(after > before, "expected activity to update (before={before}, after={after})");
}

#[cfg(feature = "dev-frontend")]
#[tokio::test]
async fn unmatched_api_path_returns_json_404_not_spa_html() {
    // `/api/bogus` must not be swallowed by the SPA fallback. Structural
    // test (router topology), gated on dev-frontend because it calls
    // `build_router_with_dist` with a seeded tempdir — embed-dist ignores
    // the dist_path argument and uses the baked-in assets instead.
    let state = /* ... build minimal AppState ... */;
    let dist = tempfile::tempdir().unwrap();
    seed_stub_dist(dist.path());
    let app = build_router_with_dist(state, dist.path().to_path_buf());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/bogus")
                .header("host", "127.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let ct = resp.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("application/json"),
        "expected JSON 404, got content-type: {ct}"
    );
}
```

All three tests seed a stub `index.html` in a tempdir and run
unconditionally — no silent-skip on missing `frontend/dist/`. The
embed-dist feature mode is excluded because `build_router_with_dist`
ignores the `dist_path` argument in that mode (the dist is baked into
the binary at compile time).

**Lock in D10 wire compression with a fourth test** — asserts that the
`CompressionLayer` emits `Content-Encoding: br` when the client advertises
brotli support. Prevents a silent regression where a future refactor
reorders or drops the layer.

```rust
#[cfg(feature = "dev-frontend")]
#[tokio::test]
async fn spa_asset_is_brotli_encoded_for_br_clients() {
    let state = /* ... build minimal AppState ... */;
    let dist = tempfile::tempdir().unwrap();
    // Seed a non-trivial asset so the CompressionLayer sees enough bytes
    // to actually compress (it skips below a minimum threshold).
    std::fs::create_dir_all(dist.path().join("assets")).unwrap();
    std::fs::write(
        dist.path().join("assets/app.js"),
        "// ".to_string() + &"x".repeat(4096),
    ).unwrap();
    std::fs::write(dist.path().join("index.html"), "<!doctype html>").unwrap();

    let app = build_router_with_dist(state, dist.path().to_path_buf());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/assets/app.js")
                .header("host", "127.0.0.1")
                .header("accept-encoding", "br")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let ce = resp.headers()
        .get("content-encoding")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(ce, "br", "expected Content-Encoding: br, got: {ce:?}");
}
```

**Update `serves_placeholder_root_and_writes_info`** — add a dist seed step and run under
`dev-frontend`:

```rust
#[cfg(feature = "dev-frontend")]
#[tokio::test]
async fn serves_spa_root_and_writes_info() {
    // Seed a stub dist dir and build the router with it — no dependency
    // on `frontend/dist/` having been built by npm. Runs unconditionally.
    let dist = tempfile::tempdir().unwrap();
    seed_stub_dist(dist.path());
    // ... build state as before; use build_router_with_dist(state, dist.path()...)
    // instead of build_router(state). Rest of test unchanged (poll
    // server-info.json, GET /, assert 200, assert body contains HTML).
}
```

### 2e. Add `pub mod assets` to `lib.rs`

```rust
pub mod assets;
```

### Success criteria

```bash
cd skills/visualisation/visualise/server

# Compile-smoke first — catches router-composition / middleware-ordering
# typechecks before the (slower) test run. Especially worth doing
# because `from_fn_with_state(state.activity, …)` applied after the
# inner `.with_state(state)` relies on subtle axum type plumbing.
cargo check --features dev-frontend

# Unit tests under dev-frontend: path_normalisation_tests (5) +
# dev_frontend_tests (3) = 8 tests. No npm build required.
cargo test --lib --features dev-frontend

# Unit tests under embed-dist: path_normalisation_tests (5) +
# embed_tests (4) = 9 tests. Uses tests/fixtures/mini-dist/ for the
# test-only embed, but the production `Frontend` struct still requires
# `../frontend/dist/` to exist at compile time (rust-embed reads folders
# during `#[derive(Embed)]`), so this command requires `npm run build`
# to have run first.
cargo test --lib

cargo test --tests --features dev-frontend
# spa_serving::tests::* — runs unconditionally against a seeded tempdir
# (no longer depends on `npm run build` having populated frontend/dist/)
```

---

## Step 3: Frontend scaffold

### 3a. Add `node` to `mise.toml`

**File**: `mise.toml` — add to `[tools]`:

```toml
node = "22"
```

Add frontend test task and update the `test:unit` dependency. Follows
the existing convention: mise delegates to an invoke task, which runs
the actual command. The relevant feature-flag plumbing for cargo lives
in `tasks/test/unit.py` and `tasks/test/integration.py` (see Step 10e
for those edits; the summary is that the invoke `visualiser` tasks
pass `--features dev-frontend` so all feature-gated tests are included,
and the unit task additionally runs a second cargo invocation under
default features so the `embed_tests` module is also covered).

```toml
[tasks."test:unit:frontend"]
description = "Run visualiser frontend unit tests (Vitest)"
run = "invoke test.unit.frontend"

[tasks."test:unit"]
description = "Run all unit tests in parallel"
depends = ["test:unit:visualiser", "test:unit:frontend"]
```

Run `mise install` to install Node 22 before proceeding.

### 3b. Create `package.json`

**File**: `skills/visualisation/visualise/frontend/package.json`

```json
{
  "name": "accelerator-visualiser-frontend",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "engines": {
    "node": ">=22"
  },
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:watch": "vitest",
    "test:coverage": "vitest run --coverage"
  },
  "dependencies": {
    "@tanstack/react-query": "^5",
    "@tanstack/react-router": "^1",
    "highlight.js": "^11",
    "react": "^19",
    "react-dom": "^19",
    "react-markdown": "^9",
    "rehype-highlight": "^7",
    "remark-gfm": "^4"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6",
    "@testing-library/react": "^16",
    "@testing-library/user-event": "^14",
    "@types/react": "^19",
    "@types/react-dom": "^19",
    "@vitejs/plugin-react": "^4",
    "@vitest/coverage-v8": "^3",
    "jsdom": "^26",
    "typescript": "^5",
    "vite": "^6",
    "vitest": "^3"
  }
}
```

`highlight.js` is declared as a direct dependency because
`MarkdownRenderer` imports `highlight.js/styles/github.css` directly
(Step 8b). It is also a peer of `rehype-highlight`, so a single entry
satisfies both. `engines.node >=22` makes `npm install` warn on
contributors who skipped `mise install` and are running an older Node.

### 3c. Create `vite.config.ts`

**File**: `skills/visualisation/visualise/frontend/vite.config.ts`

```typescript
import { readFileSync } from 'node:fs'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

/**
 * Resolve the dev API port in this order:
 *   1. `VISUALISER_API_PORT` env var (explicit override).
 *   2. `VISUALISER_INFO_PATH` env var → read `{ port }` from that JSON file
 *      (typically `<tmp_path>/server-info.json` written by the Rust server).
 *   3. Give up and leave the proxy pointing at an obviously-invalid port
 *      so the failure mode is loud (ECONNREFUSED against 127.0.0.1:0)
 *      rather than silently succeeding against something unintended.
 *
 * The production/test bundles don't use the dev proxy — the SPA is served
 * from the same origin as the API — so this resolution only runs during
 * `vite dev`.
 */
function resolveApiPort(): number {
  const fromEnv = process.env.VISUALISER_API_PORT
  if (fromEnv && Number.isFinite(Number(fromEnv))) return Number(fromEnv)

  const infoPath = process.env.VISUALISER_INFO_PATH
  if (infoPath) {
    try {
      const info = JSON.parse(readFileSync(infoPath, 'utf-8')) as { port?: number }
      if (typeof info.port === 'number') return info.port
    } catch (err) {
      console.warn(
        `[vite.config] Failed to read port from VISUALISER_INFO_PATH=${infoPath}:`,
        err,
      )
    }
  }

  console.warn(
    '[vite.config] Dev API port not resolved — set VISUALISER_API_PORT=<port> ' +
    'or VISUALISER_INFO_PATH=<path to server-info.json> before `npm run dev`. ' +
    'Falling back to port 0, which will ECONNREFUSED loudly.',
  )
  return 0
}

const apiPort = resolveApiPort()

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': {
        target: `http://127.0.0.1:${apiPort}`,
        changeOrigin: true,
      },
    },
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    css: true,
    // Restore original implementations between tests so `vi.spyOn(...)`
    // doesn't leak mocked return values across `it` blocks in the same
    // file. `clearAllMocks` alone only resets call history.
    restoreMocks: true,
  },
})
```

Dev workflow for contributors:

```bash
# After starting the Rust server, point vite at the chosen port. Either:
export VISUALISER_API_PORT=47123      # explicit
# …or let vite read it from server-info.json:
export VISUALISER_INFO_PATH=/path/to/tmp/server-info.json

cd skills/visualisation/visualise/frontend
npm run dev
```

The production and test bundles don't use the dev proxy — the SPA is
served from the same origin as the API.

### 3d. Create `tsconfig.json` and `tsconfig.node.json`

**File**: `skills/visualisation/visualise/frontend/tsconfig.json`

```json
{
  "files": [],
  "references": [
    { "path": "./tsconfig.app.json" },
    { "path": "./tsconfig.node.json" }
  ]
}
```

**File**: `skills/visualisation/visualise/frontend/tsconfig.app.json`

```json
{
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.app.tsbuildinfo",
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

**File**: `skills/visualisation/visualise/frontend/tsconfig.node.json`

```json
{
  "compilerOptions": {
    "tsBuildInfoFile": "./node_modules/.tmp/tsconfig.node.tsbuildinfo",
    "target": "ES2022",
    "lib": ["ES2023"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true
  },
  "include": ["vite.config.ts"]
}
```

### 3e. Create `index.html`

**File**: `skills/visualisation/visualise/frontend/index.html`

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Accelerator Visualiser</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

### 3f. Test setup file

**File**: `skills/visualisation/visualise/frontend/src/test/setup.ts`

```typescript
import '@testing-library/jest-dom'
import { vi, beforeAll, afterAll } from 'vitest'

// Stub global EventSource as a safety net — prevents any test that
// mounts a component using the production `useDocEvents` hook from
// opening a real network connection. The SSE hook tests themselves do
// NOT depend on this stub; they use `makeUseDocEvents(fakeFactory)` to
// inject their own fake (see use-doc-events.test.ts).
class MockEventSource {
  static CONNECTING = 0
  static OPEN = 1
  static CLOSED = 2
  readyState = MockEventSource.OPEN
  onmessage: ((e: MessageEvent) => void) | null = null
  onerror: ((e: Event) => void) | null = null
  close = vi.fn()
  constructor(_url: string) {}
}

beforeAll(() => {
  vi.stubGlobal('EventSource', MockEventSource)
})

afterAll(() => {
  vi.unstubAllGlobals()
})

// `restoreMocks: true` in vite.config.ts already restores `vi.spyOn`
// mocks between tests. No explicit `afterEach(vi.clearAllMocks)` is
// needed — adding it would be redundant and might shadow future
// per-file cleanup.
```

### 3g. Install dependencies

```bash
cd skills/visualisation/visualise/frontend
npm install
```

This produces `package-lock.json` — commit it alongside `package.json`.

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
# Scaffold verification: confirm the build pipeline is wired. No
# `npm run test` here — Vitest 3.x errors on an empty test set without
# --passWithNoTests, and the first real test-run happens at Step 5
# once query-keys.test.ts and fetch.test.ts exist.
npm run build
# Should produce dist/index.html
```

---

## Step 4: TypeScript API types

**File**: `skills/visualisation/visualise/frontend/src/api/types.ts` (new)

These types mirror the server's wire format exactly. No tests needed — they are
pure type definitions verified by the TypeScript compiler.

All fields use camelCase to match the server's
`#[serde(rename_all = "camelCase")]` output. Enum-valued fields use the
kebab-case form emitted by the server's `#[serde(rename_all = "kebab-case")]`
enums.

```typescript
// All fields use camelCase to match the server's
// `#[serde(rename_all = "camelCase")]` output.

export type DocTypeKey =
  | 'decisions' | 'tickets' | 'plans' | 'research'
  | 'plan-reviews' | 'pr-reviews'
  | 'validations' | 'notes' | 'prs' | 'templates'

/** Single source of truth for the DocTypeKey union at runtime. Drives both
 *  the `isDocTypeKey` type guard and the router's `parseParams` validators,
 *  so URL params are narrowed at the routing boundary rather than inside
 *  each view component. */
export const DOC_TYPE_KEYS: readonly DocTypeKey[] = [
  'decisions', 'tickets', 'plans', 'research',
  'plan-reviews', 'pr-reviews',
  'validations', 'notes', 'prs', 'templates',
] as const

/** Type guard: narrows a string to `DocTypeKey` when valid. */
export function isDocTypeKey(s: string): s is DocTypeKey {
  return (DOC_TYPE_KEYS as readonly string[]).includes(s)
}
```

**File**: `skills/visualisation/visualise/frontend/src/api/path-utils.ts` (new)

Small helpers used by both `LibraryTypeView` (to build the `$fileSlug`
param in a route link) and `LibraryDocView` (to reverse-lookup the entry
by basename).

```typescript
/** Extract the URL-friendly fileSlug from an IndexEntry's relPath —
 *  the last path segment with its `.md` extension stripped. Server
 *  indexer only admits `.md` today, so stripping a single extension is
 *  sufficient; update both this helper and the indexer contract if
 *  other extensions are ever admitted. */
export function fileSlugFromRelPath(relPath: string): string {
  return relPath.split('/').at(-1)?.replace(/\.md$/, '') ?? relPath
}

export interface DocType {
  key: DocTypeKey
  label: string
  dirPath: string | null
  inLifecycle: boolean
  inKanban: boolean
  // Required: the server always emits this field (see Step 1b). Templates
  // and any future virtual/derived types set `virtual: true`; real
  // document types set `virtual: false`. Sidebar partitions on this flag.
  virtual: boolean
}

export interface IndexEntry {
  type: DocTypeKey
  path: string
  relPath: string
  slug: string | null
  title: string
  frontmatter: Record<string, unknown>
  frontmatterState: 'parsed' | 'absent' | 'malformed'
  ticket: string | null
  mtimeMs: number
  size: number
  etag: string
}

export interface DocsListResponse {
  docs: IndexEntry[]
}

export type TemplateTierSource = 'config-override' | 'user-override' | 'plugin-default'

export interface TemplateTier {
  source: TemplateTierSource
  path: string
  present: boolean
  active: boolean
  content?: string
  etag?: string
}

export interface TemplateSummary {
  name: string
  tiers: TemplateTier[]
  activeTier: TemplateTierSource
}

export interface TemplateSummaryListResponse {
  templates: TemplateSummary[]
}

export interface TemplateDetail {
  name: string
  tiers: TemplateTier[]
  activeTier: TemplateTierSource
}

export interface SseDocChangedEvent {
  type: 'doc-changed'
  docType: DocTypeKey
  path: string
  etag?: string
}

export interface SseDocInvalidEvent {
  type: 'doc-invalid'
  docType: DocTypeKey
  path: string
}

export type SseEvent = SseDocChangedEvent | SseDocInvalidEvent
```

---

## Step 5: Query client + SSE hook (TDD)

### 5a. Query keys

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.ts`

```typescript
import type { DocTypeKey } from './types'

export const queryKeys = {
  types: () => ['types'] as const,
  docs: (type: DocTypeKey) => ['docs', type] as const,
  docContent: (relPath: string) => ['doc-content', relPath] as const,
  templates: () => ['templates'] as const,
  templateDetail: (name: string) => ['template-detail', name] as const,
  lifecycle: () => ['lifecycle'] as const,
  lifecycleCluster: (slug: string) => ['lifecycle-cluster', slug] as const,
  kanban: () => ['kanban'] as const,
} as const
```

**File**: `skills/visualisation/visualise/frontend/src/api/query-keys.test.ts`

```typescript
import { describe, it, expect } from 'vitest'
import { queryKeys } from './query-keys'

describe('queryKeys', () => {
  it('returns stable arrays for the same inputs', () => {
    expect(queryKeys.docs('plans')).toEqual(['docs', 'plans'])
    expect(queryKeys.docContent('meta/plans/foo.md')).toEqual([
      'doc-content', 'meta/plans/foo.md',
    ])
    expect(queryKeys.templateDetail('adr')).toEqual(['template-detail', 'adr'])
  })

  it('types key is a singleton', () => {
    expect(queryKeys.types()).toEqual(['types'])
  })
})
```

### 5b. API fetch functions

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.ts`

```typescript
import type {
  DocType, DocTypeKey, DocsListResponse, IndexEntry,
  TemplateSummaryListResponse, TemplateDetail,
} from './types'

export async function fetchTypes(): Promise<DocType[]> {
  const r = await fetch('/api/types')
  if (!r.ok) throw new Error(`GET /api/types: ${r.status}`)
  return r.json()
}

export async function fetchDocs(type: DocTypeKey): Promise<IndexEntry[]> {
  const r = await fetch(`/api/docs?type=${encodeURIComponent(type)}`)
  if (!r.ok) throw new Error(`GET /api/docs?type=${type}: ${r.status}`)
  const body: DocsListResponse = await r.json()
  return body.docs
}

export async function fetchDocContent(relPath: string): Promise<{ content: string; etag: string }> {
  // Encode per-segment so filenames containing '#', '?', '%', or
  // non-ASCII are transmitted correctly. '/' separators between segments
  // stay literal since the server accepts them as path structure.
  const encodedPath = relPath.split('/').map(encodeURIComponent).join('/')
  const r = await fetch(`/api/docs/${encodedPath}`)
  if (!r.ok) throw new Error(`GET /api/docs/${relPath}: ${r.status}`)
  const content = await r.text()
  const etag = r.headers.get('etag') ?? ''
  return { content, etag }
}

export async function fetchTemplates(): Promise<TemplateSummaryListResponse> {
  const r = await fetch('/api/templates')
  if (!r.ok) throw new Error(`GET /api/templates: ${r.status}`)
  return r.json()
}

export async function fetchTemplateDetail(name: string): Promise<TemplateDetail> {
  const r = await fetch(`/api/templates/${encodeURIComponent(name)}`)
  if (!r.ok) throw new Error(`GET /api/templates/${name}: ${r.status}`)
  return r.json()
}
```

**File**: `skills/visualisation/visualise/frontend/src/api/fetch.test.ts`

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  fetchTypes, fetchDocs, fetchDocContent,
  fetchTemplates, fetchTemplateDetail,
} from './fetch'

const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)

beforeEach(() => mockFetch.mockReset())

describe('fetchTypes', () => {
  it('returns parsed JSON on 200', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => [{ key: 'decisions', label: 'Decisions', dirPath: '/p', inLifecycle: true, inKanban: false, virtual: false }],
    })
    const types = await fetchTypes()
    expect(types).toHaveLength(1)
    expect(types[0].key).toBe('decisions')
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 500 })
    await expect(fetchTypes()).rejects.toThrow('500')
  })
})

describe('fetchDocs', () => {
  it('unwraps the `docs` field from the response envelope', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ docs: [{ type: 'plans', path: '/p', relPath: 'r' }] }),
    })
    const docs = await fetchDocs('plans')
    expect(Array.isArray(docs)).toBe(true)
    expect(docs).toHaveLength(1)
  })

  it('url-encodes the type parameter', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ docs: [] }),
    })
    await fetchDocs('plan-reviews')
    expect(mockFetch).toHaveBeenCalledWith('/api/docs?type=plan-reviews')
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404 })
    await expect(fetchDocs('plans')).rejects.toThrow('404')
  })
})

describe('fetchDocContent', () => {
  it('returns content and etag', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      text: async () => '# Hello',
      headers: { get: (h: string) => h === 'etag' ? '"sha256-abc"' : null },
    })
    const result = await fetchDocContent('meta/plans/foo.md')
    expect(result.content).toBe('# Hello')
    expect(result.etag).toBe('"sha256-abc"')
  })

  it('encodes path segments individually, preserving slash separators', async () => {
    // Locks in the per-segment encoding: spaces and special characters
    // get percent-encoded within a segment, but '/' between segments
    // stays literal so the server route `/api/docs/*path` receives the
    // right structure.
    mockFetch.mockResolvedValueOnce({
      ok: true, text: async () => '', headers: { get: () => null },
    })
    await fetchDocContent('meta/plans/with spaces/file#1.md')
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/docs/meta/plans/with%20spaces/file%231.md',
    )
  })

  it('falls back to empty etag when the header is missing', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true, text: async () => 'x', headers: { get: () => null },
    })
    const result = await fetchDocContent('foo.md')
    expect(result.etag).toBe('')
  })
})

describe('fetchTemplates', () => {
  it('returns the full template-summary envelope', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ templates: [{ name: 'adr', activeTier: 'plugin-default', tiers: [] }] }),
    })
    const result = await fetchTemplates()
    expect(result.templates).toHaveLength(1)
    expect(result.templates[0].name).toBe('adr')
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 500 })
    await expect(fetchTemplates()).rejects.toThrow('500')
  })
})

describe('fetchTemplateDetail', () => {
  it('url-encodes the template name', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ name: 'adr', activeTier: 'plugin-default', tiers: [] }),
    })
    await fetchTemplateDetail('adr')
    expect(mockFetch).toHaveBeenCalledWith('/api/templates/adr')
  })

  it('throws on non-200', async () => {
    mockFetch.mockResolvedValueOnce({ ok: false, status: 404 })
    await expect(fetchTemplateDetail('missing')).rejects.toThrow('404')
  })
})
```

### 5c. TanStack Query client

**File**: `skills/visualisation/visualise/frontend/src/api/query-client.ts`

```typescript
import { QueryClient } from '@tanstack/react-query'

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // SSE (useDocEvents) is the authoritative invalidator for every
      // server-backed cache — file edits trigger doc-changed events that
      // invalidate docs/docContent/lifecycle/kanban. A time-based staleness
      // threshold would only cause redundant refetches on focus / remount.
      staleTime: Infinity,
      retry: 1,
    },
  },
})
```

### 5d. SSE hook (TDD)

Write the test first:

**File**: `skills/visualisation/visualise/frontend/src/api/use-doc-events.test.ts`

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { dispatchSseEvent, makeUseDocEvents } from './use-doc-events'
import { queryKeys } from './query-keys'

// ── Pure dispatch tests ──────────────────────────────────────────────────
// No hooks, no EventSource, no async — just exercise the invalidation
// rules directly.
describe('dispatchSseEvent', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    queryClient = new QueryClient()
    vi.spyOn(queryClient, 'invalidateQueries')
  })

  it('invalidates docs query on doc-changed event', () => {
    dispatchSseEvent(
      { type: 'doc-changed', docType: 'plans', path: 'meta/plans/foo.md', etag: 'sha256-abc' },
      queryClient,
    )
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: queryKeys.docs('plans') }),
    )
  })

  it('invalidates doc content for the changed file', () => {
    dispatchSseEvent(
      { type: 'doc-changed', docType: 'plans', path: 'meta/plans/foo.md', etag: 'sha256-abc' },
      queryClient,
    )
    // Refreshes the markdown body when the open detail view's file changes.
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: queryKeys.docContent('meta/plans/foo.md') }),
    )
  })

  it('invalidates kanban on ticket doc-changed event', () => {
    dispatchSseEvent(
      { type: 'doc-changed', docType: 'tickets', path: 'meta/tickets/0001-foo.md', etag: 'sha256-abc' },
      queryClient,
    )
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: queryKeys.kanban() }),
    )
  })
})

// ── Wiring tests via the factory ─────────────────────────────────────────
// Construct an isolated hook with a fake EventSource factory. Instance
// capture happens via a test-local closure — no global-stub coordination.
describe('makeUseDocEvents wiring', () => {
  let queryClient: QueryClient

  class FakeEventSource {
    onmessage: ((e: MessageEvent) => void) | null = null
    onerror: ((e: Event) => void) | null = null
    close = vi.fn()
    constructor(public url: string) {}
  }

  beforeEach(() => { queryClient = new QueryClient() })

  function wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(QueryClientProvider, { client: queryClient }, children)
  }

  it('opens an EventSource to /api/events', () => {
    const factory = vi.fn(
      (url: string) => new FakeEventSource(url) as unknown as EventSource,
    )
    const useDocEvents = makeUseDocEvents(factory)
    renderHook(() => useDocEvents(), { wrapper })
    expect(factory).toHaveBeenCalledWith('/api/events')
  })

  it('closes the EventSource on unmount', () => {
    let captured: FakeEventSource | null = null
    const useDocEvents = makeUseDocEvents((url) => {
      captured = new FakeEventSource(url)
      return captured as unknown as EventSource
    })
    const { unmount } = renderHook(() => useDocEvents(), { wrapper })
    unmount()
    expect(captured!.close).toHaveBeenCalled()
  })

  it('ignores malformed JSON without throwing or invalidating', () => {
    vi.spyOn(queryClient, 'invalidateQueries')
    vi.spyOn(console, 'warn').mockImplementation(() => {})   // silence debug log
    let captured: FakeEventSource | null = null
    const useDocEvents = makeUseDocEvents((url) => {
      captured = new FakeEventSource(url)
      return captured as unknown as EventSource
    })
    renderHook(() => useDocEvents(), { wrapper })

    expect(() => {
      captured!.onmessage?.(new MessageEvent('message', { data: 'not json' }))
    }).not.toThrow()
    expect(queryClient.invalidateQueries).not.toHaveBeenCalled()
  })

  it('invalidates all docs queries via prefix match on EventSource error', () => {
    // Seed two populated docs queries so the prefix invalidation can be
    // observed by state change, not just by call shape. This locks in
    // TanStack Query's default partial-match semantics (`exact: false`)
    // — a future global `exact: true` would break reconcile-on-reconnect
    // and this test would catch it.
    queryClient.setQueryData(queryKeys.docs('plans'), [])
    queryClient.setQueryData(queryKeys.docs('tickets'), [])
    vi.spyOn(console, 'warn').mockImplementation(() => {})  // silence debug log

    let captured: FakeEventSource | null = null
    const useDocEvents = makeUseDocEvents((url) => {
      captured = new FakeEventSource(url)
      return captured as unknown as EventSource
    })
    renderHook(() => useDocEvents(), { wrapper })

    captured!.onerror?.(new Event('error'))

    // Both child queries are marked stale.
    expect(queryClient.getQueryState(queryKeys.docs('plans'))?.isInvalidated).toBe(true)
    expect(queryClient.getQueryState(queryKeys.docs('tickets'))?.isInvalidated).toBe(true)
  })
})
```

The global `MockEventSource` stub from `src/test/setup.ts` (Step 3f) is
unchanged and stays as a safety net — any test that accidentally mounts
a component using the production `useDocEvents` will hit the stub rather
than opening a real network connection. No `lastInstance` patching is
needed on the stub because these tests capture the fake instance
directly via factory closure.

Implement the hook as a pure dispatch function + a factory that wires
an `EventSource` to it. Production exports a singleton hook built with
the real `EventSource`; tests build isolated hooks with fake factories,
or call the pure dispatch function directly.

**File**: `skills/visualisation/visualise/frontend/src/api/use-doc-events.ts`

```typescript
import { useEffect } from 'react'
import { useQueryClient, type QueryClient } from '@tanstack/react-query'
import { queryKeys } from './query-keys'
import type { SseEvent } from './types'

export type EventSourceFactory = (url: string) => EventSource

/**
 * Pure dispatch: given an SSE event, invalidate the appropriate query
 * caches. Exported so unit tests can exercise the invalidation logic
 * without any hook / EventSource / async machinery.
 *
 * `event.path` matches the `relPath` used by `fetchDocContent`, so
 * invalidating `docContent(event.path)` refreshes the rendered markdown
 * body when the currently-open detail view's file changes on disk.
 */
export function dispatchSseEvent(
  event: SseEvent,
  queryClient: QueryClient,
): void {
  if (event.type === 'doc-changed' || event.type === 'doc-invalid') {
    void queryClient.invalidateQueries({ queryKey: queryKeys.docs(event.docType) })
    void queryClient.invalidateQueries({ queryKey: queryKeys.docContent(event.path) })
    void queryClient.invalidateQueries({ queryKey: queryKeys.lifecycle() })
    if (event.docType === 'tickets') {
      void queryClient.invalidateQueries({ queryKey: queryKeys.kanban() })
    }
  }
}

/**
 * Build a `useDocEvents` hook bound to a specific EventSource factory.
 * Production wires the real `EventSource` once (see `useDocEvents` below);
 * tests construct isolated hooks with fake factories so they can capture
 * the EventSource instance via a test-local closure — no global stub
 * coordination required.
 */
export function makeUseDocEvents(createSource: EventSourceFactory) {
  return function useDocEvents(): void {
    const queryClient = useQueryClient()

    useEffect(() => {
      const source = createSource('/api/events')

      source.onmessage = (e: MessageEvent) => {
        try {
          const event = JSON.parse(e.data as string) as SseEvent
          dispatchSseEvent(event, queryClient)
        } catch (err) {
          // Malformed SSE data — don't crash the hook, but surface the
          // problem so server/client schema drift is debuggable.
          console.warn('useDocEvents: failed to parse SSE message', { data: e.data, err })
        }
      }

      // Native EventSource auto-reconnects on network errors; events
      // fired during the outage are lost. Invalidate the top-level docs
      // prefix so reconnecting clients refetch and reconcile. Full
      // reconnect UX (banner + backoff) is Phase 10.
      source.onerror = () => {
        console.warn('useDocEvents: EventSource error — invalidating docs cache')
        void queryClient.invalidateQueries({ queryKey: ['docs'] })
      }

      return () => source.close()
    }, [queryClient])
  }
}

/** Production hook. Wired once at module load. */
export const useDocEvents = makeUseDocEvents((url) => new EventSource(url))
```

### Success criteria

```bash
cd skills/visualisation/visualise/frontend
npm run test
# query-keys.test.ts (2 tests), fetch.test.ts (12 tests),
# use-doc-events.test.ts (4 tests) — all pass
```

---

## Step 6: Route tree + root layout + sidebar (TDD)

### 6a. Entry point and root

**File**: `skills/visualisation/visualise/frontend/src/main.tsx`

```typescript
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from '@tanstack/react-router'
import { QueryClientProvider } from '@tanstack/react-query'
import { router } from './router'
import { queryClient } from './api/query-client'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>,
)
```

### 6b. Router

**File**: `skills/visualisation/visualise/frontend/src/router.ts`

```typescript
import {
  createRouter,
  createRoute,
  createRootRoute,
  redirect,
} from '@tanstack/react-router'
import { RootLayout } from './components/RootLayout/RootLayout'
import { LibraryLayout } from './routes/library/LibraryLayout'
import { LibraryTypeView } from './routes/library/LibraryTypeView'
import { LibraryDocView } from './routes/library/LibraryDocView'
import { LibraryTemplatesIndex } from './routes/library/LibraryTemplatesIndex'
import { LibraryTemplatesView } from './routes/library/LibraryTemplatesView'
import { LifecycleStub } from './routes/lifecycle/LifecycleStub'
import { KanbanStub } from './routes/kanban/KanbanStub'
import { isDocTypeKey, type DocTypeKey } from './api/types'

const rootRoute = createRootRoute({ component: RootLayout })

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  beforeLoad: () => { throw redirect({ to: '/library' }) },
})

const libraryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/library',
  component: LibraryLayout,
})

// Landing at /library redirects to the Decisions index so users see
// content rather than an empty main pane.
const libraryIndexRoute = createRoute({
  getParentRoute: () => libraryRoute,
  path: '/',
  beforeLoad: () => {
    throw redirect({ to: '/library/$type', params: { type: 'decisions' } })
  },
})

// Dedicated Templates routes — literal paths beat the `/$type` param
// route below, so these are dispatched directly by the router rather
// than via a runtime `if (type === 'templates')` branch inside the
// generic views. Keeps `LibraryTypeView` / `LibraryDocView` focused on
// real document types and lets the route tree self-document.
const libraryTemplatesIndexRoute = createRoute({
  getParentRoute: () => libraryRoute,
  path: '/templates',
  component: LibraryTemplatesIndex,
})

const libraryTemplateDetailRoute = createRoute({
  getParentRoute: () => libraryRoute,
  path: '/templates/$name',
  component: LibraryTemplatesView,
})

// `parseParams` narrows `type: string` → `type: DocTypeKey` at the router
// boundary. An unknown type in the URL redirects to /library rather than
// rendering a silently-wrong view. `/templates` and `/templates/$name`
// are caught by the literal-path routes above before this route matches,
// so `parseParams` never rejects real Templates URLs.
const libraryTypeRoute = createRoute({
  getParentRoute: () => libraryRoute,
  path: '/$type',
  parseParams: (raw: Record<string, string>): { type: DocTypeKey } => {
    if (!isDocTypeKey(raw.type)) {
      throw redirect({ to: '/library' })
    }
    return { type: raw.type }
  },
  component: LibraryTypeView,
})

const libraryDocRoute = createRoute({
  getParentRoute: () => libraryTypeRoute,
  path: '/$fileSlug',
  component: LibraryDocView,
})

const lifecycleRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/lifecycle',
  component: LifecycleStub,
})

const kanbanRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/kanban',
  component: KanbanStub,
})

// Exported so tests can construct an isolated router with memory history
// (see router.test.tsx) — production uses `router` below.
export const routeTree = rootRoute.addChildren([
  indexRoute,
  libraryRoute.addChildren([
    libraryIndexRoute,
    // Dedicated Templates routes registered before the generic $type
    // route; literal-path specificity means the router matches these
    // first for `/library/templates` and `/library/templates/:name`.
    libraryTemplatesIndexRoute,
    libraryTemplateDetailRoute,
    libraryTypeRoute.addChildren([libraryDocRoute]),
  ]),
  lifecycleRoute,
  kanbanRoute,
])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
```

### 6b-tests. Router tree tests

Verify the root redirect, the `parseParams` narrowing redirect, and the
literal-path specificity of the dedicated Templates routes. Catches
routing regressions that are otherwise invisible until manual QA.

**File**: `skills/visualisation/visualise/frontend/src/router.test.tsx`

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import {
  RouterProvider, createRouter, createMemoryHistory,
} from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { routeTree } from './router'
import * as fetchModule from './api/fetch'

function renderAt(url: string) {
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [url] }),
  })
  const qc = new QueryClient()
  render(
    <QueryClientProvider client={qc}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return router
}

/** Wait for the router to settle at a specific pathname. Multi-hop
 *  redirect chains (e.g. `/` → `/library` → `/library/decisions`) require
 *  multiple re-evaluation passes that `router.load()` does not
 *  single-shot resolve; `waitFor` polls the router state until the
 *  expected destination is reached. */
async function waitForPath(
  router: ReturnType<typeof createRouter>,
  expected: string,
): Promise<void> {
  await waitFor(() => {
    expect(router.state.location.pathname).toBe(expected)
  })
}

describe('router', () => {
  // RootLayout fetches /api/types and useDocEvents opens EventSource; stub
  // network calls so routing logic is what's actually tested.
  beforeEach(() => {
    vi.spyOn(fetchModule, 'fetchTypes').mockResolvedValue([])
    vi.spyOn(fetchModule, 'fetchTemplates').mockResolvedValue({ templates: [] })
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue({
      name: 'adr', activeTier: 'plugin-default', tiers: [],
    })
  })

  it('redirects / to /library/decisions (via /library)', async () => {
    // Chain: / → /library → /library/decisions
    const router = renderAt('/')
    await waitForPath(router, '/library/decisions')
  })

  it('redirects bare /library to /library/decisions', async () => {
    const router = renderAt('/library')
    await waitForPath(router, '/library/decisions')
  })

  it('routes /library/templates to the templates index', async () => {
    const router = renderAt('/library/templates')
    await waitForPath(router, '/library/templates')
    // Heading from LibraryTemplatesIndex — matched via the literal
    // /library/templates route, not the generic /library/$type.
    expect(
      await screen.findByRole('heading', { name: 'Templates' }),
    ).toBeInTheDocument()
  })

  it('routes /library/templates/adr to the templates detail view', async () => {
    const router = renderAt('/library/templates/adr')
    await waitForPath(router, '/library/templates/adr')
    // LibraryTemplatesView heading is the template name; matched via the
    // literal /library/templates/$name route.
    expect(
      await screen.findByRole('heading', { name: 'adr' }),
    ).toBeInTheDocument()
  })

  it('redirects /library/bogus to /library/decisions when the type is unknown', async () => {
    // parseParams on libraryTypeRoute throws redirect({ to: '/library' })
    // for any string that is not a DocTypeKey; /library then chains to
    // /library/decisions.
    const router = renderAt('/library/bogus')
    await waitForPath(router, '/library/decisions')
  })
})
```

### 6c. Sidebar (TDD)

Write the test first:

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.test.tsx`

```typescript
import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from './test-helpers'
import { Sidebar } from './Sidebar'
import type { DocType } from '../../api/types'

const mockDocTypes: DocType[] = [
  { key: 'decisions', label: 'Decisions', dirPath: '/p', inLifecycle: true, inKanban: false, virtual: false },
  { key: 'tickets', label: 'Tickets', dirPath: '/p', inLifecycle: true, inKanban: true, virtual: false },
  { key: 'plans', label: 'Plans', dirPath: '/p', inLifecycle: true, inKanban: false, virtual: false },
  { key: 'templates', label: 'Templates', dirPath: null, inLifecycle: false, inKanban: false, virtual: true },
]

describe('Sidebar', () => {
  it('renders all doc type labels', () => {
    render(<MemoryRouter><Sidebar docTypes={mockDocTypes} /></MemoryRouter>)
    expect(screen.getByText('Decisions')).toBeInTheDocument()
    expect(screen.getByText('Tickets')).toBeInTheDocument()
    expect(screen.getByText('Plans')).toBeInTheDocument()
  })

  it('renders Templates under a "Meta" heading', () => {
    render(<MemoryRouter><Sidebar docTypes={mockDocTypes} /></MemoryRouter>)
    expect(screen.getByText('Meta')).toBeInTheDocument()
    expect(screen.getByText('Templates')).toBeInTheDocument()
  })

  it('renders Lifecycle and Kanban nav items', () => {
    render(<MemoryRouter><Sidebar docTypes={mockDocTypes} /></MemoryRouter>)
    expect(screen.getByText('Lifecycle')).toBeInTheDocument()
    expect(screen.getByText('Kanban')).toBeInTheDocument()
  })
})
```

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/test-helpers.tsx`

A tiny wrapper that provides a minimal router context:

```typescript
import React from 'react'
import { createRouter, createRootRoute, createMemoryHistory, RouterProvider } from '@tanstack/react-router'
import { Outlet } from '@tanstack/react-router'

export function MemoryRouter({ children }: { children: React.ReactNode }) {
  const root = createRootRoute({ component: () => <>{children}</> })
  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ['/'] }),
  })
  return <RouterProvider router={router} />
}
```

Implement the sidebar:

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.tsx`

```typescript
import { Link, useRouterState } from '@tanstack/react-router'
import type { DocType } from '../../api/types'
import styles from './Sidebar.module.css'

const VIEW_TYPES: Array<{ path: string; label: string }> = [
  { path: '/lifecycle', label: 'Lifecycle' },
  { path: '/kanban', label: 'Kanban' },
]

interface Props {
  docTypes: DocType[]
}

export function Sidebar({ docTypes }: Props) {
  const location = useRouterState({ select: s => s.location })

  // Partition on the server-provided `virtual` flag. Real doc types go
  // under Documents; virtual types (Templates today, future derived views)
  // go under Meta. No hardcoded key allow-list — the backend is the single
  // source of truth for the type taxonomy.
  const mainTypes = docTypes.filter(t => !t.virtual)
  const metaTypes = docTypes.filter(t => t.virtual)

  return (
    <nav className={styles.sidebar}>
      <section className={styles.section}>
        <h2 className={styles.sectionHeading}>Documents</h2>
        <ul className={styles.list}>
          {mainTypes.map(t => (
            <li key={t.key}>
              <Link
                to="/library/$type"
                params={{ type: t.key }}
                className={`${styles.link} ${
                  location.pathname.startsWith(`/library/${t.key}`) ? styles.active : ''
                }`}
              >
                {t.label}
              </Link>
            </li>
          ))}
        </ul>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionHeading}>Views</h2>
        <ul className={styles.list}>
          {VIEW_TYPES.map(v => (
            <li key={v.path}>
              <Link
                to={v.path}
                className={`${styles.link} ${
                  location.pathname === v.path ? styles.active : ''
                }`}
              >
                {v.label}
              </Link>
            </li>
          ))}
        </ul>
      </section>

      <section className={`${styles.section} ${styles.meta}`}>
        <h2 className={styles.sectionHeading}>Meta</h2>
        <ul className={styles.list}>
          {metaTypes.map(t => (
            <li key={t.key}>
              <Link
                to="/library/$type"
                params={{ type: t.key }}
                className={`${styles.link} ${styles.muted} ${
                  location.pathname.startsWith(`/library/${t.key}`) ? styles.active : ''
                }`}
              >
                {t.label}
              </Link>
            </li>
          ))}
        </ul>
      </section>
    </nav>
  )
}
```

**File**: `skills/visualisation/visualise/frontend/src/components/Sidebar/Sidebar.module.css`

```css
.sidebar {
  width: 220px;
  min-height: 100vh;
  padding: 1rem 0;
  border-right: 1px solid #e5e7eb;
  background: #f9fafb;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.section { padding: 0 0.75rem; }

.sectionHeading {
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #6b7280;
  margin: 0 0 0.4rem;
}

.list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }

.link {
  display: block;
  padding: 0.3rem 0.6rem;
  border-radius: 4px;
  font-size: 0.875rem;
  color: #374151;
  text-decoration: none;
}
.link:hover { background: #e5e7eb; }
.active { background: #dbeafe; color: #1d4ed8; font-weight: 500; }

.meta .sectionHeading { color: #9ca3af; }
.muted { color: #6b7280; }
```

### 6d. Root layout

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.tsx`

```typescript
import { Outlet } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Sidebar } from '../Sidebar/Sidebar'
import { useDocEvents } from '../../api/use-doc-events'
import { fetchTypes } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import styles from './RootLayout.module.css'

export function RootLayout() {
  useDocEvents()

  const { data: docTypes = [] } = useQuery({
    queryKey: queryKeys.types(),
    queryFn: fetchTypes,
  })

  return (
    <div className={styles.shell}>
      <Sidebar docTypes={docTypes} />
      <main className={styles.main}>
        <Outlet />
      </main>
    </div>
  )
}
```

**File**: `skills/visualisation/visualise/frontend/src/components/RootLayout/RootLayout.module.css`

```css
.shell { display: flex; min-height: 100vh; font-family: system-ui, sans-serif; }
.main { flex: 1; overflow: auto; padding: 1.5rem 2rem; }
```

### 6e. Stub views for lifecycle and kanban

**File**: `skills/visualisation/visualise/frontend/src/routes/lifecycle/LifecycleStub.tsx`

```typescript
export function LifecycleStub() {
  return <p>Lifecycle view — coming in Phase 6.</p>
}
```

**File**: `skills/visualisation/visualise/frontend/src/routes/kanban/KanbanStub.tsx`

```typescript
export function KanbanStub() {
  return <p>Kanban view — coming in Phase 7.</p>
}
```

### Success criteria

```bash
npm run test
# Sidebar.test.tsx: 3 tests pass
```

---

## Step 7: Library index table (TDD)

### 7a. Test first

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.test.tsx`

```typescript
import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'
import { LibraryTypeView } from './LibraryTypeView'
import * as fetchModule from '../../api/fetch'
import type { IndexEntry } from '../../api/types'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'

const mockEntries: IndexEntry[] = [
  {
    type: 'plans', path: '/p/meta/plans/2026-01-01-foo.md',
    relPath: 'meta/plans/2026-01-01-foo.md',
    slug: 'foo', title: 'Foo Plan',
    frontmatter: { status: 'draft', date: '2026-01-01' },
    frontmatterState: 'parsed', ticket: null,
    mtimeMs: 1_700_000_000_000, size: 100, etag: 'sha256-a',
  },
  {
    type: 'plans', path: '/p/meta/plans/2026-02-01-bar.md',
    relPath: 'meta/plans/2026-02-01-bar.md',
    slug: 'bar', title: 'Bar Plan',
    frontmatter: { status: 'complete', date: '2026-02-01' },
    frontmatterState: 'parsed', ticket: null,
    mtimeMs: 1_700_100_000_000, size: 200, etag: 'sha256-b',
  },
]

function Wrapper({ children }: { children: React.ReactNode }) {
  // Disable retries in tests so rejected fetches surface as error state
  // immediately instead of re-firing the mock and slowing the suite.
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('LibraryTypeView', () => {
  it('renders a row for each doc', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue(mockEntries)
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    expect(await screen.findByText('Foo Plan')).toBeInTheDocument()
    expect(screen.getByText('Bar Plan')).toBeInTheDocument()
  })

  it('clicking a column header sorts the table', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue(mockEntries)
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    await screen.findByText('Foo Plan')

    fireEvent.click(screen.getByRole('button', { name: /title/i }))
    const rows = screen.getAllByRole('row').slice(1) // skip header
    expect(rows[0]).toHaveTextContent('Bar Plan')
  })

  it('toggles sort direction on repeated clicks of the same column', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue(mockEntries)
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    await screen.findByText('Foo Plan')

    const titleButton = screen.getByRole('button', { name: /title/i })
    fireEvent.click(titleButton)  // first click: ascending
    let rows = screen.getAllByRole('row').slice(1)
    expect(rows[0]).toHaveTextContent('Bar Plan')

    fireEvent.click(titleButton)  // second click: descending
    rows = screen.getAllByRole('row').slice(1)
    expect(rows[0]).toHaveTextContent('Foo Plan')
  })

  it('shows empty-state message when no docs', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([])
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    expect(await screen.findByText(/No documents found/i)).toBeInTheDocument()
  })

  it('shows loading state while fetching', () => {
    // Never-resolving promise keeps isLoading=true for the lifetime of the test.
    vi.spyOn(fetchModule, 'fetchDocs').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    expect(screen.getByText(/Loading…/i)).toBeInTheDocument()
  })

  it('renders an error branch for an unknown doc type', () => {
    // Bypass the DocTypeKey type with a cast — the point is to exercise
    // the runtime narrowing introduced by Finding 11.
    render(<LibraryTypeView type={'bogus' as never} />, { wrapper: Wrapper })
    expect(screen.getByRole('alert')).toHaveTextContent(/Unknown doc type/i)
  })

  it('renders a fetch-error alert when fetchDocs rejects', async () => {
    // retry: 1 in queryClient config means the query retries once before
    // settling as errored. Tests that don't want to wait override retry
    // via QueryClient defaultOptions at the Wrapper level. Here we just
    // wait for the eventual error render.
    vi.spyOn(fetchModule, 'fetchDocs').mockRejectedValue(new Error('boom'))
    render(<LibraryTypeView type="plans" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load documents/i)
  })
})
```

Note: `LibraryTypeView` must accept a `type` prop in tests but read it from the router
params when used inside the route tree. The test passes `type` directly.

### 7b. Implement `LibraryTypeView`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.tsx`

```typescript
import { useMemo, useState } from 'react'
import { Link, useParams } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchDocs } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import type { IndexEntry, DocTypeKey } from '../../api/types'
import { isDocTypeKey } from '../../api/types'
import { fileSlugFromRelPath } from '../../api/path-utils'
import styles from './LibraryTypeView.module.css'

type SortKey = 'title' | 'slug' | 'status' | 'mtime'
type SortDir = 'asc' | 'desc'

/** Extract the status-cell's displayed value, matching the fallback
 *  chain in the rendered cell. Sort contract: clicking a column header
 *  orders rows by what the user sees in that column. */
function statusCellValue(entry: IndexEntry): string {
  const fm = entry.frontmatter as Record<string, unknown>
  return String(fm.status ?? fm.date ?? '')
}

function sortEntries(entries: IndexEntry[], key: SortKey, dir: SortDir): IndexEntry[] {
  return [...entries].sort((a, b) => {
    let av: string | number, bv: string | number
    if (key === 'title') { av = a.title; bv = b.title }
    else if (key === 'slug') { av = a.slug ?? ''; bv = b.slug ?? '' }
    else if (key === 'status') {
      av = statusCellValue(a)
      bv = statusCellValue(b)
    }
    else { av = a.mtimeMs; bv = b.mtimeMs }
    if (av < bv) return dir === 'asc' ? -1 : 1
    if (av > bv) return dir === 'asc' ? 1 : -1
    return 0
  })
}

interface Props { type?: DocTypeKey }

export function LibraryTypeView({ type: propType }: Props) {
  // Prop takes precedence (for tests that render the component directly);
  // otherwise read from the router. The route's `parseParams` (see
  // router.ts) has already narrowed the URL param to DocTypeKey — the
  // `isDocTypeKey` check below is belt-and-braces for the prop path.
  const params = useParams({ strict: false }) as { type?: string }
  const rawType = propType ?? params.type

  const [sortKey, setSortKey] = useState<SortKey>('mtime')
  const [sortDir, setSortDir] = useState<SortDir>('desc')

  // Narrowed only when rawType passes the type guard; undefined otherwise.
  // Used both to gate the query (avoids a pointless fetch on invalid
  // input) and to render the error branch below.
  const type: DocTypeKey | undefined =
    rawType && isDocTypeKey(rawType) ? rawType : undefined

  // Call useQuery unconditionally (Rules of Hooks). When `type` is invalid
  // we disable the query and render an error; the key uses a sentinel so
  // the invalid case does not share a cache entry with a real type.
  const { data: entries = [], isLoading, isError, error } = useQuery({
    queryKey: type ? queryKeys.docs(type) : ['docs', '__invalid__'] as const,
    queryFn: () => fetchDocs(type!),
    enabled: type !== undefined,
  })

  function toggleSort(key: SortKey) {
    if (sortKey === key) setSortDir(d => d === 'asc' ? 'desc' : 'asc')
    else { setSortKey(key); setSortDir('asc') }
  }

  // Memoise BEFORE any conditional early returns — Rules of Hooks require
  // every hook to run in the same order on every render. Memoising an
  // empty array during the invalid/loading branches is cheap.
  const sorted = useMemo(
    () => sortEntries(entries, sortKey, sortDir),
    [entries, sortKey, sortDir],
  )

  const ariaSortFor = (key: SortKey): 'ascending' | 'descending' | 'none' =>
    sortKey === key ? (sortDir === 'asc' ? 'ascending' : 'descending') : 'none'

  if (type === undefined) {
    return <p role="alert">Unknown doc type: {String(rawType)}</p>
  }
  if (isLoading) return <p>Loading…</p>
  if (isError) {
    return (
      <p role="alert" className={styles.error}>
        Failed to load documents: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  }

  return (
    <div className={styles.container}>
      <table className={styles.table}>
        <thead>
          <tr>
            <SortHeader label="Title"    skey="title"  ariaSort={ariaSortFor('title')}  onToggle={toggleSort} current={sortKey} dir={sortDir} />
            <SortHeader label="Status"   skey="status" ariaSort={ariaSortFor('status')} onToggle={toggleSort} current={sortKey} dir={sortDir} />
            <SortHeader label="Slug"     skey="slug"   ariaSort={ariaSortFor('slug')}   onToggle={toggleSort} current={sortKey} dir={sortDir} />
            <SortHeader label="Modified" skey="mtime"  ariaSort={ariaSortFor('mtime')}  onToggle={toggleSort} current={sortKey} dir={sortDir} />
          </tr>
        </thead>
        <tbody>
          {sorted.map(entry => (
            <tr key={entry.relPath}>
              <td>
                <Link to="/library/$type/$fileSlug" params={{ type, fileSlug: fileSlugFromRelPath(entry.relPath) }}>
                  {entry.title}
                </Link>
              </td>
              <td>
                <span className={styles.badge}>
                  {statusCellValue(entry) || '—'}
                </span>
              </td>
              <td className={styles.slug}>{entry.slug ?? '—'}</td>
              <td className={styles.mtime}>
                {formatMtime(entry.mtimeMs)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {entries.length === 0 && <p className={styles.empty}>No documents found.</p>}
    </div>
  )
}

/** Sortable column header. `<button>` semantics give keyboard users
 *  Enter/Space to toggle; `aria-sort` communicates current state to
 *  assistive technologies. */
function SortHeader({
  label, skey, ariaSort, current, dir, onToggle,
}: {
  label: string
  skey: SortKey
  ariaSort: 'ascending' | 'descending' | 'none'
  current: SortKey
  dir: SortDir
  onToggle: (k: SortKey) => void
}) {
  const isActive = current === skey
  const arrow = isActive ? (dir === 'asc' ? ' ▲' : ' ▼') : ''
  return (
    <th aria-sort={ariaSort}>
      <button
        type="button"
        className={styles.sortButton}
        onClick={() => onToggle(skey)}
      >
        {label}{arrow}
      </button>
    </th>
  )
}

/** Format recently-modified times as a short relative string ("5m ago"),
 *  older times as a full date-time string. Users care about the minute
 *  when a live edit just landed, and the date when browsing older docs. */
function formatMtime(ms: number): string {
  const diffMs = Date.now() - ms
  const diffSec = Math.floor(diffMs / 1000)
  if (diffSec < 60)              return `${diffSec}s ago`
  if (diffSec < 3600)            return `${Math.floor(diffSec / 60)}m ago`
  if (diffSec < 24 * 3600)       return `${Math.floor(diffSec / 3600)}h ago`
  return new Date(ms).toLocaleString()
}
```

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTypeView.module.css`

```css
.container { max-width: 900px; }
.table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
.table th {
  text-align: left; padding: 0; /* padding is on .sortButton */
  border-bottom: 2px solid #e5e7eb;
}
.sortButton {
  all: unset;
  display: block;
  width: 100%;
  padding: 0.5rem 0.75rem;
  font: inherit;
  font-weight: 600;
  color: #374151;
  cursor: pointer;
  user-select: none;
}
.sortButton:hover,
.sortButton:focus-visible {
  color: #1d4ed8;
  outline: 2px solid #1d4ed8;
  outline-offset: -2px;
}
.table td { padding: 0.4rem 0.75rem; border-bottom: 1px solid #f3f4f6; }
.table tr:hover td { background: #f9fafb; }
.badge {
  display: inline-block; padding: 0.1rem 0.45rem;
  font-size: 0.75rem; border-radius: 9999px;
  background: #e5e7eb; color: #374151;
}
.slug { color: #6b7280; font-size: 0.8rem; font-family: monospace; }
.mtime { color: #9ca3af; font-size: 0.8rem; white-space: nowrap; }
.empty { color: #6b7280; margin-top: 2rem; }
.error {
  color: #991b1b; background: #fef2f2; border: 1px solid #fecaca;
  border-radius: 4px; padding: 0.5rem 0.75rem; margin-top: 1rem;
}
```

### 7c. Library layout

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryLayout.tsx`

```typescript
import { Outlet } from '@tanstack/react-router'

export function LibraryLayout() {
  return <Outlet />
}
```

### Success criteria

```bash
npm run test
# LibraryTypeView.test.tsx: 7 tests pass (render + sort asc + sort toggle
# + empty state + loading state + unknown-type error + fetch-error alert)
```

---

## Step 8: Library doc detail + markdown rendering (TDD)

### 8a. FrontmatterChips component (TDD)

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.test.tsx`

```typescript
import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { FrontmatterChips } from './FrontmatterChips'

describe('FrontmatterChips', () => {
  it('renders key-value pairs from frontmatter', () => {
    render(
      <FrontmatterChips
        frontmatter={{ status: 'draft', date: '2026-01-01', author: 'Toby' }}
        state="parsed"
      />
    )
    expect(screen.getByText(/status/i)).toBeInTheDocument()
    expect(screen.getByText('draft')).toBeInTheDocument()
  })

  it('renders a warning banner for malformed frontmatter', () => {
    render(<FrontmatterChips frontmatter={{}} state="malformed" />)
    expect(screen.getByRole('alert')).toBeInTheDocument()
  })

  it('renders nothing for absent frontmatter (no error, no chips)', () => {
    const { container } = render(<FrontmatterChips frontmatter={{}} state="absent" />)
    expect(container.firstChild).toBeNull()
  })
})
```

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.tsx`

```typescript
import styles from './FrontmatterChips.module.css'

interface Props {
  frontmatter: Record<string, unknown>
  state: 'parsed' | 'absent' | 'malformed'
}

export function FrontmatterChips({ frontmatter, state }: Props) {
  if (state === 'absent') return null

  if (state === 'malformed') {
    return (
      <div role="alert" className={styles.banner}>
        Frontmatter unparseable — showing raw content.
      </div>
    )
  }

  const entries = Object.entries(frontmatter).filter(
    ([, v]) => v !== null && v !== undefined,
  )

  if (entries.length === 0) return null

  return (
    <dl className={styles.chips}>
      {entries.map(([k, v]) => (
        <div key={k} className={styles.chip}>
          <dt className={styles.key}>{k}</dt>
          <dd className={styles.value}>{formatChipValue(v)}</dd>
        </div>
      ))}
    </dl>
  )
}

/** Render arbitrary YAML frontmatter values as readable text. Arrays
 *  join with commas; nested objects JSON-stringify rather than showing
 *  the useless "[object Object]" default from `String(obj)`. */
function formatChipValue(v: unknown): string {
  if (Array.isArray(v)) return v.join(', ')
  if (v !== null && typeof v === 'object') return JSON.stringify(v)
  return String(v)
}
```

**File**: `skills/visualisation/visualise/frontend/src/components/FrontmatterChips/FrontmatterChips.module.css`

```css
.chips { display: flex; flex-wrap: wrap; gap: 0.4rem; margin: 0 0 1rem; }
.chip {
  display: flex; gap: 0.25rem; align-items: baseline;
  background: #f3f4f6; border-radius: 4px;
  padding: 0.2rem 0.5rem; font-size: 0.78rem;
}
.key { font-weight: 600; color: #374151; margin: 0; }
.value { color: #6b7280; margin: 0; }
.banner {
  background: #fef3c7; border: 1px solid #f59e0b;
  border-radius: 4px; padding: 0.5rem 0.75rem;
  font-size: 0.875rem; margin-bottom: 1rem;
}
```

### 8b. MarkdownRenderer component (TDD)

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.test.tsx`

```typescript
import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MarkdownRenderer } from './MarkdownRenderer'

describe('MarkdownRenderer', () => {
  it('renders headings', () => {
    render(<MarkdownRenderer content="# Hello World" />)
    expect(screen.getByRole('heading', { level: 1, name: 'Hello World' })).toBeInTheDocument()
  })

  it('renders GFM tables', async () => {
    render(<MarkdownRenderer content="| A | B |\n|---|---|\n| x | y |" />)
    expect(await screen.findByRole('table')).toBeInTheDocument()
  })

  it('renders a code block', () => {
    render(<MarkdownRenderer content="```js\nconsole.log('hi')\n```" />)
    expect(screen.getByText(/console\.log/)).toBeInTheDocument()
  })

  it('renders paragraphs', () => {
    render(<MarkdownRenderer content="Hello paragraph." />)
    expect(screen.getByText('Hello paragraph.')).toBeInTheDocument()
  })

  it('does not render raw HTML (XSS regression guard)', () => {
    // react-markdown defaults to escaping HTML. This test locks in that
    // default so enabling `rehype-raw` or `allowDangerousHtml` in future
    // requires deliberately breaking this test — at which point the
    // contributor is forced to add a sanitiser (e.g. rehype-sanitize).
    const { container } = render(
      <MarkdownRenderer content="<script>alert('xss')</script>" />,
    )
    expect(container.querySelector('script')).toBeNull()
    // The raw text survives as content; it's just not parsed as HTML.
    expect(container.textContent).toContain("<script>alert('xss')</script>")
  })

  it('does not render javascript: URLs in links (XSS regression guard)', () => {
    const { container } = render(
      <MarkdownRenderer content="[click]( javascript:alert(1) )" />,
    )
    const anchor = container.querySelector('a')
    // react-markdown's default urlTransform strips/rewrites dangerous schemes.
    // We assert no anchor with a javascript: href makes it into the DOM.
    expect(anchor?.getAttribute('href') ?? '').not.toMatch(/^\s*javascript:/i)
  })
})
```

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.tsx`

```typescript
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import rehypeHighlight from 'rehype-highlight'
import styles from './MarkdownRenderer.module.css'

interface Props {
  content: string
}

export function MarkdownRenderer({ content }: Props) {
  return (
    <div className={styles.markdown}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
      >
        {content}
      </ReactMarkdown>
    </div>
  )
}
```

**File**: `skills/visualisation/visualise/frontend/src/components/MarkdownRenderer/MarkdownRenderer.module.css`

```css
.markdown {
  max-width: 720px;
  line-height: 1.6;
  color: #1f2937;
}
.markdown h1, .markdown h2, .markdown h3 {
  margin-top: 1.5rem; margin-bottom: 0.5rem; font-weight: 600;
}
.markdown h1 { font-size: 1.75rem; border-bottom: 1px solid #e5e7eb; padding-bottom: 0.3rem; }
.markdown h2 { font-size: 1.35rem; }
.markdown h3 { font-size: 1.1rem; }
.markdown p { margin: 0.75rem 0; }
.markdown pre {
  background: #1e1e1e; color: #d4d4d4;
  border-radius: 6px; padding: 1rem;
  overflow-x: auto; font-size: 0.85rem;
}
.markdown code:not(pre code) {
  background: #f3f4f6; border-radius: 3px;
  padding: 0.1rem 0.3rem; font-size: 0.88em;
}
.markdown table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
.markdown th, .markdown td { border: 1px solid #e5e7eb; padding: 0.4rem 0.75rem; }
.markdown th { background: #f9fafb; font-weight: 600; }
.markdown blockquote {
  border-left: 4px solid #d1d5db; margin: 1rem 0;
  padding: 0.5rem 1rem; color: #6b7280;
}
```

You will need to import a highlight.js theme CSS in `src/main.tsx` or as a global import:

```typescript
// In main.tsx, add:
import 'highlight.js/styles/github.css'
```

### 8c. LibraryDocView (TDD)

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.test.tsx`

```typescript
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryDocView } from './LibraryDocView'
import * as fetchModule from '../../api/fetch'
import type { IndexEntry } from '../../api/types'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'

const mockEntry: IndexEntry = {
  type: 'plans', path: '/p/meta/plans/2026-01-01-foo.md',
  relPath: 'meta/plans/2026-01-01-foo.md',
  slug: 'foo', title: 'Foo Plan',
  frontmatter: { status: 'draft' }, frontmatterState: 'parsed', ticket: null,
  mtimeMs: 1_700_000_000_000, size: 100, etag: 'sha256-a',
}

function Wrapper({ children }: { children: React.ReactNode }) {
  // Disable retries in tests so rejected fetches surface as error state
  // immediately instead of re-firing the mock and slowing the suite.
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('LibraryDocView', () => {
  it('renders the doc title', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Foo Plan\nBody text.',
      etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('Foo Plan')).toBeInTheDocument()
  })

  it('renders frontmatter chips', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Title', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('draft')).toBeInTheDocument()
  })

  it('renders the markdown body', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockResolvedValue({
      content: '# Foo Plan\nBody text here.', etag: '"sha256-a"',
    })
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByText('Body text here.')).toBeInTheDocument()
  })

  it('shows Document not found when the slug does not match any entry', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    render(<LibraryDocView type="plans" fileSlug="nonexistent" />, { wrapper: Wrapper })
    expect(await screen.findByText(/Document not found/i)).toBeInTheDocument()
  })

  it('renders Loading while the content fetch is pending', () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(screen.getByText(/Loading…/i)).toBeInTheDocument()
  })

  it('renders an error alert when fetchDocs rejects', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockRejectedValue(new Error('list-boom'))
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load document list/i)
  })

  it('renders an error alert when fetchDocContent rejects', async () => {
    vi.spyOn(fetchModule, 'fetchDocs').mockResolvedValue([mockEntry])
    vi.spyOn(fetchModule, 'fetchDocContent').mockRejectedValue(new Error('content-boom'))
    render(<LibraryDocView type="plans" fileSlug="2026-01-01-foo" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load document content/i)
  })
})
```

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.tsx`

```typescript
import { useParams } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchDocs, fetchDocContent } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { FrontmatterChips } from '../../components/FrontmatterChips/FrontmatterChips'
import { MarkdownRenderer } from '../../components/MarkdownRenderer/MarkdownRenderer'
import type { DocTypeKey } from '../../api/types'
import { isDocTypeKey } from '../../api/types'
import { fileSlugFromRelPath } from '../../api/path-utils'
import styles from './LibraryDocView.module.css'

interface Props {
  type?: DocTypeKey
  fileSlug?: string
}

export function LibraryDocView({ type: propType, fileSlug: propSlug }: Props) {
  // See LibraryTypeView for the prop-or-param + narrowing rationale.
  const params = useParams({ strict: false }) as { type?: string; fileSlug?: string }
  const rawType = propType ?? params.type
  const fileSlug = propSlug ?? params.fileSlug ?? ''

  const type: DocTypeKey | undefined =
    rawType && isDocTypeKey(rawType) ? rawType : undefined

  const { data: entries = [], isError: listError, error: listErr } = useQuery({
    queryKey: type ? queryKeys.docs(type) : ['docs', '__invalid__'] as const,
    queryFn: () => fetchDocs(type!),
    enabled: type !== undefined,
  })

  const entry = entries.find(e => fileSlugFromRelPath(e.relPath) === fileSlug)

  const { data: docContent, isLoading, isError: contentError, error: contentErr } = useQuery({
    queryKey: queryKeys.docContent(entry?.relPath ?? ''),
    queryFn: () => fetchDocContent(entry!.relPath),
    enabled: !!entry,
  })

  if (type === undefined) {
    return <p role="alert">Unknown doc type: {String(rawType)}</p>
  }
  if (!fileSlug) {
    return <p role="alert">Missing file slug.</p>
  }
  if (listError) {
    return <p role="alert" className={styles.error}>
      Failed to load document list: {listErr instanceof Error ? listErr.message : String(listErr)}
    </p>
  }
  if (contentError) {
    return <p role="alert" className={styles.error}>
      Failed to load document content: {contentErr instanceof Error ? contentErr.message : String(contentErr)}
    </p>
  }
  if (!entry && entries.length > 0) return <p>Document not found.</p>
  if (isLoading || !docContent) return <p>Loading…</p>

  return (
    <article className={styles.article}>
      <header className={styles.header}>
        <h1 className={styles.title}>{entry!.title}</h1>
        <FrontmatterChips
          frontmatter={entry!.frontmatter as Record<string, unknown>}
          state={entry!.frontmatterState}
        />
      </header>

      <div className={styles.aside}>
        <section>
          <h3>Related artifacts</h3>
          <p className={styles.empty}>No related artifacts yet.</p>
        </section>
        <section>
          <h3>File</h3>
          <p className={styles.meta}>{entry!.relPath}</p>
        </section>
      </div>

      <div className={styles.body}>
        <MarkdownRenderer content={docContent.content} />
      </div>
    </article>
  )
}
```

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.module.css`

```css
.article {
  display: grid;
  grid-template-areas: "header header" "body aside";
  grid-template-columns: 1fr 260px;
  gap: 1.5rem 2rem;
  max-width: 1100px;
}
.header { grid-area: header; }
.title { font-size: 1.6rem; font-weight: 700; margin: 0 0 0.5rem; }
.body { grid-area: body; }
.aside { grid-area: aside; font-size: 0.85rem; color: #6b7280; }
.aside h3 { font-size: 0.75rem; font-weight: 600; text-transform: uppercase; color: #9ca3af; margin: 0 0 0.4rem; }
.aside section { margin-bottom: 1.5rem; }
.empty { font-style: italic; }
.meta { word-break: break-all; font-family: monospace; font-size: 0.78rem; }
.error {
  color: #991b1b; background: #fef2f2; border: 1px solid #fecaca;
  border-radius: 4px; padding: 0.5rem 0.75rem; margin-top: 1rem;
}
```

### Success criteria

```bash
npm run test
# FrontmatterChips.test.tsx: 3 tests pass
# MarkdownRenderer.test.tsx: 4 tests pass
# LibraryDocView.test.tsx: 7 tests pass (title + chips + body + not-found
# + loading + list-error + content-error)
```

---

## Step 9: Templates view (TDD)

### 9a. Test first

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.test.tsx`

```typescript
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryTemplatesView } from './LibraryTemplatesView'
import * as fetchModule from '../../api/fetch'
import type { TemplateDetail } from '../../api/types'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'

const mockDetail: TemplateDetail = {
  name: 'adr',
  activeTier: 'plugin-default',
  tiers: [
    { source: 'config-override', path: '/no-config', present: false, active: false },
    { source: 'user-override',   path: '/meta/templates/adr.md', present: false, active: false },
    { source: 'plugin-default',  path: '/plugin/templates/adr.md', present: true, active: true,
      content: '# ADR\nBody.', etag: 'sha256-x' },
  ],
}

function Wrapper({ children }: { children: React.ReactNode }) {
  // Disable retries in tests so rejected fetches surface as error state
  // immediately instead of re-firing the mock and slowing the suite.
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('LibraryTemplatesView', () => {
  it('renders a panel for each tier', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByText(/plugin.default/i)).toBeInTheDocument()
    expect(screen.getByText(/config.override/i)).toBeInTheDocument()
    expect(screen.getByText(/user.override/i)).toBeInTheDocument()
  })

  it('marks the active tier', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByText('active')).toBeInTheDocument()
  })

  it('renders absent tiers as greyed-out cards with a note', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    await screen.findByText(/plugin.default/i)
    expect(screen.getAllByText(/not currently configured/i).length).toBeGreaterThanOrEqual(1)
  })

  it('renders the markdown content of the active tier', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockResolvedValue(mockDetail)
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByText('Body.')).toBeInTheDocument()
  })

  it('renders an error alert when fetchTemplateDetail rejects', async () => {
    vi.spyOn(fetchModule, 'fetchTemplateDetail').mockRejectedValue(new Error('boom'))
    render(<LibraryTemplatesView name="adr" />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load template/i)
  })
})
```

### 9b. Implement `LibraryTemplatesView`

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.tsx`

```typescript
import { useParams } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplateDetail } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import { MarkdownRenderer } from '../../components/MarkdownRenderer/MarkdownRenderer'
import type { TemplateTier } from '../../api/types'
import styles from './LibraryTemplatesView.module.css'

const TIER_LABELS: Record<string, string> = {
  'plugin-default': 'Plugin default',
  'user-override': 'User override',
  'config-override': 'Config override',
}

interface Props { name?: string }

export function LibraryTemplatesView({ name: propName }: Props) {
  // Reads from the dedicated `/library/templates/$name` route; keeps the
  // prop override so tests can render without mounting the router.
  const params = useParams({ strict: false }) as { name?: string }
  const name = propName ?? params.name

  const { data, isLoading, isError, error } = useQuery({
    queryKey: name ? queryKeys.templateDetail(name) : ['template-detail', '__invalid__'] as const,
    queryFn: () => fetchTemplateDetail(name!),
    enabled: !!name,
  })

  if (!name) {
    return <p role="alert">Missing template name.</p>
  }
  if (isError) {
    return (
      <p role="alert" className={styles.error}>
        Failed to load template: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  }
  if (isLoading || !data) return <p>Loading…</p>

  return (
    <div className={styles.container}>
      <h1 className={styles.title}>{name}</h1>
      <div className={styles.tiers}>
        {data.tiers.map(tier => (
          <TierPanel
            key={tier.source}
            tier={tier}
            isActive={tier.source === data.activeTier}
          />
        ))}
      </div>
    </div>
  )
}

/** Single source of truth for "active" is the summary's `activeTier` field.
 *  We ignore the per-tier `active` flag to avoid inconsistency if the
 *  server's tier-scan and summary-computation ever race. */
function TierPanel({ tier, isActive }: { tier: TemplateTier; isActive: boolean }) {
  return (
    <section className={`${styles.panel} ${!tier.present ? styles.absent : ''}`}>
      <header className={styles.panelHeader}>
        <span className={styles.tierLabel}>{TIER_LABELS[tier.source] ?? tier.source}</span>
        {isActive && <span className={styles.activeBadge}>active</span>}
        <code className={styles.path}>{tier.path}</code>
      </header>
      {tier.present && tier.content != null ? (
        <MarkdownRenderer content={tier.content} />
      ) : (
        <p className={styles.absent}>Not currently configured.</p>
      )}
    </section>
  )
}
```

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesView.module.css`

```css
.container { max-width: 900px; }
.title { font-size: 1.5rem; font-weight: 700; margin: 0 0 1.5rem; }
.tiers { display: flex; flex-direction: column; gap: 1.5rem; }
.panel { border: 1px solid #e5e7eb; border-radius: 8px; padding: 1rem; }
.panel.absent { opacity: 0.55; background: #f9fafb; }
.panelHeader { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem; }
.tierLabel { font-weight: 600; font-size: 0.875rem; color: #374151; }
.activeBadge {
  font-size: 0.7rem; background: #dbeafe; color: #1d4ed8;
  border-radius: 9999px; padding: 0.1rem 0.5rem; font-weight: 600;
}
.path { font-size: 0.75rem; color: #6b7280; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.error {
  color: #991b1b; background: #fef2f2; border: 1px solid #fecaca;
  border-radius: 4px; padding: 0.5rem 0.75rem; margin-top: 1rem;
}
```

### 9c. Templates index component (TDD)

Create `LibraryTemplatesIndex` — the component is mounted directly by the
dedicated `/library/templates` route added in `router.ts` (see Step 6b).
No dispatch plumbing is needed inside `LibraryTypeView` / `LibraryDocView`;
those stay focused on real document types.

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.test.tsx`

```typescript
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { LibraryTemplatesIndex } from './LibraryTemplatesIndex'
import * as fetchModule from '../../api/fetch'
import type { TemplateSummary } from '../../api/types'
import { MemoryRouter } from '../../components/Sidebar/test-helpers'

const mockTemplates: TemplateSummary[] = [
  { name: 'adr',    activeTier: 'plugin-default', tiers: [] },
  { name: 'ticket', activeTier: 'user-override',  tiers: [] },
]

function Wrapper({ children }: { children: React.ReactNode }) {
  // Disable retries in tests so rejected fetches surface as error state
  // immediately instead of re-firing the mock and slowing the suite.
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{children}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('LibraryTemplatesIndex', () => {
  it('renders a link for each template name', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates')
      .mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    expect(await screen.findByRole('link', { name: 'adr' })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'ticket' })).toBeInTheDocument()
  })

  it('renders the active tier (friendly label) beside each template name', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates')
      .mockResolvedValue({ templates: mockTemplates })
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    await screen.findByRole('link', { name: 'adr' })
    expect(screen.getByText('Plugin default')).toBeInTheDocument()
    expect(screen.getByText('User override')).toBeInTheDocument()
  })

  it('shows loading state while fetching', () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockImplementation(
      () => new Promise(() => { /* pending forever */ }),
    )
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    expect(screen.getByText(/Loading…/i)).toBeInTheDocument()
  })

  it('renders an error alert when fetchTemplates rejects', async () => {
    vi.spyOn(fetchModule, 'fetchTemplates').mockRejectedValue(new Error('boom'))
    render(<LibraryTemplatesIndex />, { wrapper: Wrapper })
    expect(await screen.findByRole('alert')).toHaveTextContent(/Failed to load templates/i)
  })
})
```

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.tsx`

```typescript
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchTemplates } from '../../api/fetch'
import { queryKeys } from '../../api/query-keys'
import type { TemplateSummary, TemplateTierSource } from '../../api/types'
import styles from './LibraryTemplatesIndex.module.css'

// Shared with LibraryTemplatesView (Step 9b). Consider lifting to a
// dedicated module if a third consumer appears.
const TIER_LABELS: Record<TemplateTierSource, string> = {
  'plugin-default':  'Plugin default',
  'user-override':   'User override',
  'config-override': 'Config override',
}

export function LibraryTemplatesIndex() {
  const { data, isLoading, isError, error } = useQuery({
    queryKey: queryKeys.templates(),
    queryFn: fetchTemplates,
  })

  if (isError) {
    return (
      <p role="alert" className={styles.error}>
        Failed to load templates: {error instanceof Error ? error.message : String(error)}
      </p>
    )
  }
  if (isLoading || !data) return <p>Loading…</p>

  return (
    <div className={styles.container}>
      <h1>Templates</h1>
      <ul className={styles.list}>
        {data.templates.map((t: TemplateSummary) => (
          <li key={t.name}>
            <Link to="/library/templates/$name" params={{ name: t.name }}>
              {t.name}
            </Link>
            <span className={styles.active}>{TIER_LABELS[t.activeTier]}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}
```

**File**: `skills/visualisation/visualise/frontend/src/routes/library/LibraryTemplatesIndex.module.css`

```css
.container { max-width: 600px; }
.list { list-style: none; padding: 0; display: flex; flex-direction: column; gap: 0.5rem; }
.list li { display: flex; align-items: center; gap: 1rem; }
.active { font-size: 0.75rem; color: #6b7280; }
.error {
  color: #991b1b; background: #fef2f2; border: 1px solid #fecaca;
  border-radius: 4px; padding: 0.5rem 0.75rem; margin-top: 1rem;
}
```

Sidebar linking: the `to="/library/$type"` link with `params={{ type: 'templates' }}`
resolves to `/library/templates`, which TanStack Router matches against the
dedicated `libraryTemplatesIndexRoute` (literal path specificity wins over
the `$type` param route). No sidebar changes needed.

### Success criteria

```bash
npm run test
# LibraryTemplatesView.test.tsx: 5 tests pass (panels + active + absent
# + markdown + fetch-error alert)
# LibraryTemplatesIndex.test.tsx: 4 tests pass (links + labels + loading
# + fetch-error alert)
```

---

## Step 10: Build verification + mise.toml update

### 10a. Build the frontend

```bash
cd skills/visualisation/visualise/frontend
npm run build
# Produces dist/index.html and dist/assets/
```

### 10b. Verify Rust build embeds the frontend

```bash
cd skills/visualisation/visualise/server
cargo build 2>&1 | grep -c error   # must output 0
```

If the build fails with "frontend/dist/index.html not found", re-run `npm run build` first
(the `build.rs` guard enforces this ordering).

### 10c. Verify dev-frontend mode compiles

```bash
# Compile-only check that the dev-frontend feature still builds cleanly
# (rust-analyzer runs this in the background, so it should always be
# green — included here as an explicit smoke step).
cargo check --manifest-path skills/visualisation/visualise/server/Cargo.toml \
  --features dev-frontend

# End-to-end dev-mode run (requires a real project config — skip on a
# fresh clone):
# ACCELERATOR_VISUALISER_BIN=target/debug/accelerator-visualiser \
#   CARGO_FEATURE_DEV_FRONTEND=1 ../scripts/launch-server.sh
```

### 10d. Run full test suite

```bash
# Frontend:
cd skills/visualisation/visualise/frontend
npm run test

# Server unit (dev-frontend feature to avoid embed-dist guard):
cd ../server
cargo test --lib --features dev-frontend

# Server integration:
cargo test --tests --features dev-frontend
```

### 10e. Update invoke tasks with feature flags and new frontend task

Three edits so `mise run test:unit` and `mise run test:integration` cover
the full suite without contributors needing to know about feature flags.

**File**: `tasks/test/unit.py` — update `visualiser` and add `frontend`
plus a shared `ensure_frontend_dist` helper:

```python
from invoke import Context, task

from .helpers import repo_root


def _ensure_frontend_dist(context: Context) -> None:
    """Build `frontend/dist/` if its index.html is missing.

    Encodes the npm-build-before-embed-dist-cargo ordering dependency
    explicitly in the task graph rather than leaving it to `build.rs`'s
    panic message. Cheap no-op when already built (vite's incremental
    build is fast).
    """
    frontend_root = repo_root() / "skills/visualisation/visualise/frontend"
    dist_index = frontend_root / "dist" / "index.html"
    if not dist_index.exists():
        context.run(f"npm --prefix {frontend_root} run build")


@task
def visualiser(context: Context):
    """Unit tests for the visualiser server.

    Runs cargo test twice to cover both feature-gated test modules:
      1. `--features dev-frontend` — covers `path_normalisation_tests`
         and `dev_frontend_tests` (ServeDir-based SPA serving). Does not
         require the SPA to be built.
      2. default features (embed-dist) — covers `path_normalisation_tests`
         and `embed_tests` (rust-embed based SPA serving). This invocation
         requires `frontend/dist/index.html` because rust-embed reads the
         folder at compile time and `build.rs` guards the index — so we
         build it first if missing.
    """
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(f"cargo test --manifest-path {manifest} --lib --features dev-frontend")
    _ensure_frontend_dist(context)
    context.run(f"cargo test --manifest-path {manifest} --lib")


@task
def frontend(context: Context):
    """Unit tests for the visualiser frontend (Vitest)."""
    frontend_root = repo_root() / "skills/visualisation/visualise/frontend"
    context.run(f"npm --prefix {frontend_root} run test")
```

**File**: `tasks/test/integration.py` — update `visualiser`:

```python
@task
def visualiser(context: Context):
    """Integration tests for the visualiser (cargo --tests + shell suites).

    The `spa_serving.rs` integration test is gated on the `dev-frontend`
    feature (uses `build_router_with_dist` with a seeded tempdir), so the
    cargo invocation must enable that feature to include it.
    """
    manifest = repo_root() / "skills/visualisation/visualise/server/Cargo.toml"
    context.run(f"cargo test --manifest-path {manifest} --tests --features dev-frontend")
    run_shell_suites(context, "skills/visualisation/visualise")
```

The `mise.toml` `test:unit:frontend` task from Step 3a already delegates
to `invoke test.unit.frontend`, so no further mise edits are needed.

---

## Full success criteria

### Automated verification

- [ ] All unit tests pass: `mise run test:unit` (runs visualiser server
      twice — once with `--features dev-frontend`, once with default
      features — plus the frontend Vitest suite).
- [ ] All integration tests pass: `mise run test:integration` (runs
      visualiser `cargo test --tests --features dev-frontend` plus shell
      suites for config and decisions).
- [ ] Full suite: `mise run test` passes.
- [ ] TypeScript compiles: `npm run build` exits 0
- [ ] `cargo build` (embed-dist) succeeds after `npm run build`

### Manual verification

- [ ] Open `http://localhost:<port>` → app loads, no console errors.
- [ ] Sidebar shows all 9 main doc types + Templates under "Meta" + Lifecycle + Kanban.
- [ ] Click "Decisions" → index table loads with all ADRs.
- [ ] Click an ADR row → doc detail page renders markdown, frontmatter chips visible.
- [ ] Click "Templates" → template list (5 names) renders.
- [ ] Click "adr" template → three-tier panel; plugin default has content; absent tiers say "Not currently configured."
- [ ] Click "Lifecycle" → stub page renders without error.
- [ ] Edit a `.md` file on disk → within ~500ms the table entry updates (SSE invalidation).
- [ ] Deep link: navigate directly to `/library/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding` → renders without refresh.

---

## Implementation sequence

Implement in this order within a single session:

1. [ ] `Cargo.toml` — add features, `rust-embed`, `mime_guess`, `tower-http fs` + compression features.
2. [ ] `server/src/docs.rs` — remove `skip_serializing_if` on `DocType.virtual` so the field always serialises.
3. [ ] `server/build.rs` — implement guard.
4. [ ] `server/src/assets.rs` — tests (path normalisation + embed + dev-frontend) then implementation.
5. [ ] `server/src/lib.rs` — add `pub mod assets`.
6. [ ] `server/src/server.rs` — replace `placeholder_root` with `apply_spa_serving`.
7. [ ] `server/tests/spa_serving.rs` — integration test.
8. [ ] Update `tasks/test/unit.py` visualiser task to run cargo twice (dev-frontend + default); update `tasks/test/integration.py` visualiser task to add `--features dev-frontend`.
9. [ ] `mise run test:unit:visualiser` and `mise run test:integration:visualiser` — must pass (runs both feature sets for unit; dev-frontend for integration).
10. [ ] `mise.toml` — add `node = "22"`, `test:unit:frontend` task delegating to `invoke test.unit.frontend`; update `test:unit` depends.
11. [ ] `mise install` — installs Node 22.
12. [ ] Frontend scaffold — `package.json`, `tsconfig*.json`, `vite.config.ts`, `index.html`, `src/test/setup.ts`.
13. [ ] `npm install` — generates `package-lock.json`.
14. [ ] `src/api/types.ts` — wire-format types.
15. [ ] `src/api/query-keys.ts` + `query-keys.test.ts`.
16. [ ] `src/api/fetch.ts` + `fetch.test.ts`.
17. [ ] `src/api/query-client.ts`.
18. [ ] `src/api/use-doc-events.ts` + `use-doc-events.test.ts`.
19. [ ] Add `frontend` invoke task to `tasks/test/unit.py`.
20. [ ] `mise run test:unit:frontend` — tests pass so far.
21. [ ] `src/components/Sidebar/` — test, implement, CSS.
22. [ ] `src/components/RootLayout/` — layout + CSS.
23. [ ] `src/routes/lifecycle/LifecycleStub.tsx`, `src/routes/kanban/KanbanStub.tsx`.
24. [ ] `src/router.ts` (with exported `routeTree`), `src/main.tsx`, `src/router.test.tsx`.
25. [ ] `mise run test:unit:frontend` — Sidebar + router tree tests pass.
26. [ ] `src/routes/library/LibraryTypeView.tsx` + test + CSS.
27. [ ] `mise run test:unit:frontend` — LibraryTypeView tests pass.
28. [ ] `src/components/FrontmatterChips/` — test, implement, CSS.
29. [ ] `src/components/MarkdownRenderer/` — test, implement, CSS.
30. [ ] `src/routes/library/LibraryDocView.tsx` + test + CSS.
31. [ ] `mise run test:unit:frontend` — FrontmatterChips, MarkdownRenderer, LibraryDocView tests pass.
32. [ ] `src/routes/library/LibraryTemplatesView.tsx` + test + CSS.
33. [ ] `src/routes/library/LibraryTemplatesIndex.tsx` + test + CSS.
34. [ ] Extend `router.ts` with dedicated `libraryTemplatesIndexRoute` + `libraryTemplateDetailRoute`.
35. [ ] `mise run test` — all tests pass.
36. [ ] `npm run build` — `dist/` produced.
37. [ ] `cargo build` — embed-dist succeeds.
38. [ ] Manual smoke test in browser.

Stop after each numbered step and verify tests pass before proceeding.

---

## References

- Spec: `meta/specs/2026-04-17-meta-visualisation-design.md` §§ Frontend, Views,
  Data model, Distribution (D10).
- Research: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
  §§ Phase 5, D9, D10.
- Phase 4 plan: `meta/plans/2026-04-22-meta-visualiser-phase-4-sse-hub-and-notify-watcher.md`
  (SSE event shapes, AppState pattern).
- Server API shape: `server/src/api/docs.rs`, `server/src/api/templates.rs`,
  `server/src/api/types.rs`.
- Wire-format types: `server/src/docs.rs`, `server/src/indexer.rs`,
  `server/src/templates.rs`.
