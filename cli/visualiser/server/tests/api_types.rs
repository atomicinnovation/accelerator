use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn types_returns_every_doc_type_with_virtual_flag_on_templates() {
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
    assert_eq!(arr.len(), 15);
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
    let design_inventories = arr
        .iter()
        .find(|t| t["key"] == "design-inventories")
        .unwrap();
    assert_eq!(design_inventories["virtual"], false);
    assert_eq!(design_inventories["inLifecycle"], true);
    assert!(design_inventories["dirPath"].is_string());

    // RCA: browsable (non-virtual), out of the linear lifecycle, mapped to a
    // real dir, and counted from the single seeded fixture.
    let rca = arr
        .iter()
        .find(|t| t["key"] == "root-cause-analyses")
        .unwrap();
    assert_eq!(rca["virtual"], false);
    assert_eq!(rca["inLifecycle"], false);
    assert!(rca["dirPath"].is_string());
    assert_eq!(rca["count"].as_u64().unwrap(), 1);

    assert_eq!(decisions["count"].as_u64().unwrap(), 1);
    let plans = arr.iter().find(|t| t["key"] == "plans").unwrap();
    assert_eq!(plans["count"].as_u64().unwrap(), 1);
    let plan_reviews = arr.iter().find(|t| t["key"] == "plan-reviews").unwrap();
    assert_eq!(plan_reviews["count"].as_u64().unwrap(), 1);
    let work_items = arr.iter().find(|t| t["key"] == "work-items").unwrap();
    assert_eq!(work_items["count"].as_u64().unwrap(), 0);
    assert_eq!(templates["count"].as_u64().unwrap(), 0);
}

async fn served_types() -> Vec<serde_json::Value> {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let state = AppState::build(cfg, Arc::new(Activity::new()))
        .await
        .unwrap();
    let res = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/types")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["types"].as_array().unwrap().clone()
}

fn set_of<'a>(
    types: &'a [serde_json::Value],
    key: &str,
) -> &'a serde_json::Value {
    &types.iter().find(|t| t["key"] == key).unwrap()["set"]
}

#[tokio::test]
async fn a_topic_research_type_carries_its_set_shape_and_presentation() {
    let types = served_types().await;

    assert_eq!(
        set_of(&types, "topic-research"),
        &serde_json::json!({
            "root": "manifest.md",
            "documentSlots": [
                "brief.md",
                "outline.md",
                "synthesis.md",
                "findings/*.md",
                "findings/*.levels/*.md",
                "reports/*.md"
            ],
            "assetSlots": [],
            "presentation": "set-page"
        })
    );
}

#[tokio::test]
async fn a_design_inventories_type_carries_its_set_shape_and_presentation() {
    let types = served_types().await;

    assert_eq!(
        set_of(&types, "design-inventories"),
        &serde_json::json!({
            "root": "inventory.md",
            "documentSlots": [],
            "assetSlots": ["screenshots/*.png"],
            "presentation": "entry-point"
        })
    );
}

#[tokio::test]
async fn every_type_that_is_not_a_set_carries_no_set() {
    let types = served_types().await;

    for doc_type in &types {
        if doc_type["key"] != "topic-research"
            && doc_type["key"] != "design-inventories"
        {
            assert!(doc_type["set"].is_null(), "{}", doc_type["key"]);
            assert!(
                doc_type.as_object().unwrap().contains_key("set"),
                "{}",
                doc_type["key"]
            );
        }
    }
}
