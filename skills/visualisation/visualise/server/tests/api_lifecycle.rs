use std::sync::Arc;

use visualiser::activity::Activity;
use visualiser::server::{build_router, AppState};
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

    let last = foo["lastChangedMs"]
        .as_i64()
        .expect("lastChangedMs missing");
    assert!(last > 0, "expected a positive mtime, got {last}");

    for entry in foo["entries"].as_array().unwrap() {
        let preview = entry["bodyPreview"].as_str().unwrap_or_else(|| {
            panic!("bodyPreview should be a string: {entry}")
        });
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
        let preview = entry["bodyPreview"].as_str().unwrap_or_else(|| {
            panic!("bodyPreview should be a string: {entry}")
        });
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
    // Lifecycle cluster: Plan + PlanReview share slug "foo" and merge.
    let lifecycle = arr
        .iter()
        .find(|c| c["completeness"]["hasPlan"] == true)
        .expect("lifecycle cluster present");
    assert!(lifecycle["entries"].as_array().unwrap().len() >= 2);
    assert_eq!(lifecycle["completeness"]["hasPlan"], true);
    // Decisions are orphan-by-design (post-Phase-4): they form their own
    // per-path bucket and don't merge with lifecycle slug-mates.
    let decision_cluster = arr
        .iter()
        .find(|c| c["completeness"]["hasDecision"] == true)
        .expect("decision cluster present");
    assert_eq!(decision_cluster["completeness"]["hasDecision"], true);
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

#[tokio::test]
async fn work_item_review_with_path_target_appears_in_work_item_cluster() {
    // End-to-end Phase 4 guard: a work-item-review whose `target:`
    // points at a work-item by path joins the work-item's cluster via
    // the typed-linkage chain.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("meta/work")).unwrap();
    std::fs::create_dir_all(root.join("meta/reviews/work")).unwrap();
    std::fs::write(
        root.join("meta/work/0099-ac2-coverage.md"),
        "---\nwork_item_id: \"0099\"\ntitle: AC2 Coverage\n---\n",
    )
    .unwrap();
    std::fs::write(
        root.join("meta/reviews/work/0099-ac2-coverage-review-1.md"),
        "---\ntarget: meta/work/0099-ac2-coverage.md\n---\n",
    )
    .unwrap();

    let cfg = common::seeded_cfg(root);
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/lifecycle/ac2-coverage")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(v["clusterKey"], "0099");
    let kinds: Vec<&str> = v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["type"].as_str())
        .collect();
    assert!(kinds.contains(&"work-items"), "kinds={kinds:?}");
    assert!(kinds.contains(&"work-item-reviews"), "kinds={kinds:?}");
}
