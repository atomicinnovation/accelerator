use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn lifecycle_list_carries_last_changed_ms_and_body_preview() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let foo = v["clusters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["slug"] == "foo")
        .unwrap();

    let last = foo["lastChangedMs"].as_i64().expect("lastChangedMs missing");
    assert!(last > 0, "expected a positive mtime, got {last}");

    for entry in foo["entries"].as_array().unwrap() {
        let preview = entry["bodyPreview"]
            .as_str()
            .unwrap_or_else(|| panic!("bodyPreview should be a string: {entry}"));
        assert_eq!(
            preview, "",
            "expected empty preview for heading-only seeded body, got {preview:?}",
        );
    }
}

#[tokio::test]
async fn lifecycle_detail_carries_last_changed_ms_and_body_preview() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle/foo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let last = v["lastChangedMs"].as_i64().expect("lastChangedMs missing");
    assert!(last > 0, "expected a positive mtime, got {last}");
    for entry in v["entries"].as_array().unwrap() {
        let preview = entry["bodyPreview"]
            .as_str()
            .unwrap_or_else(|| panic!("bodyPreview should be a string: {entry}"));
        assert_eq!(preview, "");
    }
}

#[tokio::test]
async fn lifecycle_list_groups_entries_by_slug() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["clusters"].as_array().unwrap();
    let foo = arr.iter().find(|c| c["slug"] == "foo").unwrap();
    assert!(foo["entries"].as_array().unwrap().len() >= 2);
    assert_eq!(foo["completeness"]["hasPlan"], true);
    assert_eq!(foo["completeness"]["hasDecision"], true);
}

#[tokio::test]
async fn lifecycle_one_returns_single_cluster_by_slug() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle/foo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["slug"], "foo");
}

#[tokio::test]
async fn lifecycle_unknown_slug_is_404() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
