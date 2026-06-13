use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

async fn request(uri: &str, app: axum::Router) -> serde_json::Value {
    let res = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn library_structure_returns_phases_in_canonical_order_and_top_level_templates(
) {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let v = request("/api/library/structure", app).await;

    let phases = v["phases"].as_array().unwrap();
    let ids: Vec<&str> =
        phases.iter().map(|p| p["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        vec!["define", "discover", "build", "ship", "operate", "remember"]
    );
    assert!(v["templates"].is_object());
    assert_eq!(v["templates"]["id"], "templates");
}

#[tokio::test]
async fn library_structure_operate_phase_contains_root_cause_analyses() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let v = request("/api/library/structure", app).await;

    let operate = v["phases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "operate")
        .expect("Operate phase must be emitted");
    assert_eq!(operate["label"], "Operate");
    let rca = operate["docTypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "root-cause-analyses")
        .expect("Operate phase must contain the RCA doc type");
    // The seeded cfg has exactly one RCA, so count + latest are deterministic.
    assert_eq!(rca["count"], 1);
    assert_eq!(rca["filteredCount"], 1);
    assert_eq!(rca["latest"]["title"], "Example RCA");
}

#[tokio::test]
async fn library_structure_doc_type_counts_match_counts_by_type() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let v = request("/api/library/structure", app).await;

    let remember = v["phases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "remember")
        .unwrap();
    let decisions = remember["docTypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "decisions")
        .unwrap();
    assert_eq!(decisions["count"], 1);
    assert_eq!(decisions["filteredCount"], 1);
}

#[tokio::test]
async fn library_structure_filtered_count_equals_count_with_no_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let v = request("/api/library/structure", app).await;
    for phase in v["phases"].as_array().unwrap() {
        for dt in phase["docTypes"].as_array().unwrap() {
            assert_eq!(
                dt["count"], dt["filteredCount"],
                "doc-type {:?}",
                dt["id"]
            );
        }
    }
}

#[tokio::test]
async fn library_structure_latest_is_null_for_zero_count_doc_types() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let v = request("/api/library/structure", app).await;
    // Work-items has zero entries in the seeded cfg.
    let define = v["phases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "define")
        .unwrap();
    let work_items = define["docTypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "work-items")
        .unwrap();
    assert_eq!(work_items["count"], 0);
    assert!(work_items["latest"].is_null());

    let remember = v["phases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "remember")
        .unwrap();
    let decisions = remember["docTypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "decisions")
        .unwrap();
    assert!(decisions["latest"].is_object());
}

#[tokio::test]
async fn library_structure_emits_facets_per_doc_type() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg_with_work_items(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let v = request("/api/library/structure", app).await;

    let phases = v["phases"].as_array().unwrap();
    let work_items = phases.iter().find(|p| p["id"] == "define").unwrap()
        ["docTypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "work-items")
        .unwrap()
        .clone();
    let facets: Vec<&str> = work_items["filterFacets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap())
        .collect();
    assert_eq!(facets, vec!["status", "project", "clusterSlug"]);

    let decisions = phases.iter().find(|p| p["id"] == "remember").unwrap()
        ["docTypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "decisions")
        .unwrap()
        .clone();
    let dec_facets: Vec<&str> = decisions["filterFacets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap())
        .collect();
    assert_eq!(dec_facets, vec!["status", "clusterSlug"]);

    assert!(v["templates"]["filterFacets"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn library_structure_selection_round_trip_through_query_string() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg_with_work_items(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);

    let v = request(
        "/api/library/structure?selection%5Bwork-items%5D%5Bstatus%5D=todo",
        app,
    )
    .await;

    let work_items = v["phases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "define")
        .unwrap()["docTypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "work-items")
        .unwrap()
        .clone();

    assert_eq!(work_items["count"], 3);
    assert_eq!(work_items["filteredCount"], 1);
}
