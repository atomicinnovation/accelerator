use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn docs_list_returns_index_entries_for_decisions() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=decisions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["docs"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["type"], "decisions");
    assert!(arr[0]["etag"].as_str().unwrap().starts_with("sha256-"));
}

#[tokio::test]
async fn docs_list_rejects_templates_virtual_type() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn docs_list_rejects_unknown_type() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=whatever")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn doc_fetch_returns_body_with_strong_etag() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let rel = "meta/decisions/ADR-0001-foo.md";
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/docs/{rel}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let etag = res
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(etag.contains("sha256-"));

    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/docs/{rel}"))
                .header(header::IF_NONE_MATCH, etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn doc_fetch_rejects_path_escape() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/../etc/passwd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn doc_fetch_returns_404_for_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/meta/decisions/ADR-9999-nonexistent.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn doc_fetch_handles_percent_encoded_path() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/meta/decisions/ADR%2D0001%2Dfoo.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn docs_list_for_work_items_carries_frontmatter_status() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg_with_work_items(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=work-items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let docs = v["docs"].as_array().expect("docs array");

    let by_status: std::collections::HashMap<&str, &str> =
        std::collections::HashMap::from([
            ("todo", "0001-todo-fixture"),
            ("done", "0002-done-fixture"),
            ("blocked", "0003-other-fixture"),
        ]);
    for (status, slug_prefix) in &by_status {
        let entry = docs
            .iter()
            .find(|d| {
                d["relPath"]
                    .as_str()
                    .is_some_and(|p| p.contains(slug_prefix))
            })
            .unwrap_or_else(|| panic!("expected work item {slug_prefix}"));
        assert_eq!(
            entry["frontmatter"]["status"].as_str(),
            Some(*status),
            "status pass-through broken for {slug_prefix}: got {entry}",
        );
        assert_eq!(
            entry["frontmatterState"].as_str(),
            Some("parsed"),
            "frontmatterState should be `parsed` for {slug_prefix}: got {entry}",
        );
    }
}

#[tokio::test]
async fn docs_list_returns_empty_array_for_type_with_no_files() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=work-items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["docs"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn docs_list_serialises_each_entry_id() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    std::fs::write(
        tmp.path().join("meta/plans/2026-04-19-identified.md"),
        "---\nid: \"2026-04-19-identified\"\n---\n",
    )
    .unwrap();
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=plans")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let docs = v["docs"].as_array().unwrap();
    let id_at = |rel_path: &str| {
        docs.iter()
            .find(|d| d["relPath"] == rel_path)
            .unwrap_or_else(|| panic!("no entry at {rel_path}: {docs:?}"))
            .get("id")
            .cloned()
    };
    assert_eq!(
        id_at("meta/plans/2026-04-19-identified.md"),
        Some(serde_json::json!("2026-04-19-identified"))
    );
    assert_eq!(
        id_at("meta/plans/2026-04-18-foo.md"),
        Some(serde_json::Value::Null)
    );
}

const TOPICS: &str = "meta/research/topics";

fn write_files(root: &std::path::Path, files: &[(&str, &str)]) {
    for (file, content) in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
}

fn set_cfg(tmp: &std::path::Path) -> accelerator_visualiser::config::Config {
    let mut cfg = common::seeded_cfg(tmp);
    cfg.doc_paths
        .insert("research_topics".into(), tmp.join(TOPICS));
    write_files(
        tmp,
        &[
            (
                "meta/research/topics/s/manifest.md",
                "---\nid: \"s\"\ntitle: S\n---\n",
            ),
            (
                "meta/research/topics/s/findings/01-a.md",
                "---\ntitle: A\n---\nThe finding body.\n",
            ),
            ("meta/research/topics/s/notes.txt", "notes"),
            ("meta/research/topics/s/.conduct-run.json", "{}"),
            (
                "meta/research/topics/rootless/findings/01-a.md",
                "---\ntitle: Orphan\n---\n",
            ),
            (
                "meta/research/design-inventories/i/inventory.md",
                "---\nid: \"i\"\ntitle: Inv\nstatus: accepted\n---\nBody.\n",
            ),
            (
                "meta/research/design-inventories/i/screenshots/a.png",
                "png",
            ),
        ],
    );
    cfg
}

async fn fetch_status_and_body(
    cfg: accelerator_visualiser::config::Config,
    rel: &str,
) -> (StatusCode, Vec<u8>) {
    let state = AppState::build(cfg, Arc::new(Activity::new()))
        .await
        .unwrap();
    fetch_from(state, rel).await
}

async fn fetch_from(state: Arc<AppState>, rel: &str) -> (StatusCode, Vec<u8>) {
    let res = build_router(state)
        .oneshot(
            Request::builder()
                .uri(format!("/api/docs/{rel}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn a_member_document_is_served() {
    let tmp = tempfile::tempdir().unwrap();
    let (status, body) = fetch_status_and_body(
        set_cfg(tmp.path()),
        "meta/research/topics/s/findings/01-a.md",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"---\ntitle: A\n---\nThe finding body.\n");
}

#[tokio::test]
async fn an_inventory_root_is_served_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let (status, body) = fetch_status_and_body(
        set_cfg(tmp.path()),
        "meta/research/design-inventories/i/inventory.md",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        b"---\nid: \"i\"\ntitle: Inv\nstatus: accepted\n---\nBody.\n"
    );
}

#[tokio::test]
async fn what_a_set_directory_holds_beyond_its_documents_is_not_found() {
    for rel in [
        "meta/research/design-inventories/i/screenshots/a.png",
        "meta/research/topics/s/notes.txt",
        "meta/research/topics/s/.conduct-run.json",
        "meta/research/topics/rootless/findings/01-a.md",
        "meta/research/topics/s/findings",
        "meta/research/topics/s",
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let (status, _) = fetch_status_and_body(set_cfg(tmp.path()), rel).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{rel}");
    }
}

#[tokio::test]
async fn a_directory_under_a_flat_doc_root_is_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("meta/plans/sub")).unwrap();

    let (status, _) =
        fetch_status_and_body(common::seeded_cfg(tmp.path()), "meta/plans/sub")
            .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[cfg(unix)]
#[tokio::test]
async fn an_unslotted_file_reached_through_a_symlinked_ancestor_is_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = set_cfg(tmp.path());
    std::os::unix::fs::symlink(
        tmp.path().join(TOPICS),
        tmp.path().join("meta/topics-alias"),
    )
    .unwrap();

    let (status, _) =
        fetch_status_and_body(cfg, "meta/topics-alias/s/notes.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_case_variant_path_to_a_working_file_is_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = set_cfg(tmp.path());
    let variant = "meta/Research/Topics/s/.conduct-run.json";
    if !tmp.path().join(variant).exists() {
        eprintln!("skipped: the temporary directory is case-sensitive");
        return;
    }

    let (status, _) = fetch_status_and_body(cfg, variant).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_unslotted_file_under_a_set_root_created_after_startup_is_not_found()
{
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = common::seeded_cfg(tmp.path());
    cfg.doc_paths
        .insert("research_topics".into(), tmp.path().join(TOPICS));
    let state = AppState::build(cfg, Arc::new(Activity::new()))
        .await
        .unwrap();
    write_files(
        tmp.path(),
        &[
            ("meta/research/topics/s/manifest.md", "---\ntitle: S\n---\n"),
            ("meta/research/topics/s/notes.txt", "notes"),
        ],
    );

    let (status, _) =
        fetch_from(state, "meta/research/topics/s/notes.txt").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_set_member_cannot_be_written() {
    let tmp = tempfile::tempdir().unwrap();
    let state = AppState::build(set_cfg(tmp.path()), Arc::new(Activity::new()))
        .await
        .unwrap();
    let rel = "meta/research/topics/s/findings/01-a.md";
    let etag = format!(
        "\"{}\"",
        accelerator_visualiser::file_driver::etag_of(
            &std::fs::read(tmp.path().join(rel)).unwrap()
        )
    );

    let res = build_router(state)
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/docs/{rel}/frontmatter"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::IF_MATCH, etag)
                .body(Body::from(r#"{"patch":{"status":"done"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"], "only work-items are writable");
}
