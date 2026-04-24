use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn types_returns_ten_entries_with_virtual_flag_on_templates() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(Request::builder().uri("/api/types").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["types"].as_array().unwrap();
    assert_eq!(arr.len(), 10);
    let templates = arr.iter().find(|t| t["key"] == "templates").unwrap();
    assert_eq!(templates["virtual"], true);
    assert!(templates["dirPath"].is_null());
    let decisions = arr.iter().find(|t| t["key"] == "decisions").unwrap();
    assert_eq!(decisions["virtual"], false);
    assert!(decisions["dirPath"].is_string());
}
