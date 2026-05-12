use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn types_returns_thirteen_entries_with_virtual_flag_on_templates() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/types")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["types"].as_array().unwrap();
    assert_eq!(arr.len(), 13);
    let templates = arr.iter().find(|t| t["key"] == "templates").unwrap();
    assert_eq!(templates["virtual"], true);
    assert!(templates["dirPath"].is_null());
    let decisions = arr.iter().find(|t| t["key"] == "decisions").unwrap();
    assert_eq!(decisions["virtual"], false);
    assert!(decisions["dirPath"].is_string());
    let design_gaps = arr.iter().find(|t| t["key"] == "design-gaps").unwrap();
    assert_eq!(design_gaps["virtual"], false);
    assert_eq!(design_gaps["inLifecycle"], true);
    assert!(design_gaps["dirPath"].is_string());
    let design_inventories = arr.iter().find(|t| t["key"] == "design-inventories").unwrap();
    assert_eq!(design_inventories["virtual"], false);
    assert_eq!(design_inventories["inLifecycle"], true);
    assert!(design_inventories["dirPath"].is_string());

    assert_eq!(decisions["count"].as_u64().unwrap(), 1);
    let plans = arr.iter().find(|t| t["key"] == "plans").unwrap();
    assert_eq!(plans["count"].as_u64().unwrap(), 1);
    let plan_reviews = arr.iter().find(|t| t["key"] == "plan-reviews").unwrap();
    assert_eq!(plan_reviews["count"].as_u64().unwrap(), 1);
    let work_items = arr.iter().find(|t| t["key"] == "work-items").unwrap();
    assert_eq!(work_items["count"].as_u64().unwrap(), 0);
    assert_eq!(templates["count"].as_u64().unwrap(), 0);
}
