use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn healthz_returns_200() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok\n");
}

/// Root path serves the SPA. Under dev-frontend, use `build_router_with_dist`
/// with a seeded tempdir so this runs without `npm run build` having produced
/// `frontend/dist/`.
#[cfg(feature = "dev-frontend")]
#[tokio::test]
async fn root_serves_spa_index() {
    use accelerator_visualiser::server::build_router_with_dist;

    let tmp = tempfile::tempdir().unwrap();
    let dist = tempfile::tempdir().unwrap();
    std::fs::write(
        dist.path().join("index.html"),
        "<!doctype html><html>app</html>",
    )
    .unwrap();

    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router_with_dist(state, dist.path().to_path_buf());

    let res = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header("host", "127.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let body = std::str::from_utf8(&body).unwrap();
    assert!(
        body.contains("<!doctype html") || body.contains("<!DOCTYPE html"),
        "expected HTML at /, got: {body:.200}",
    );
}

#[tokio::test]
async fn host_header_guard_still_rejects_foreign_hosts() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let req = Request::builder()
        .uri("/api/healthz")
        .header("host", "example.com")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}
