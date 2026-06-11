use std::path::Path;
use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

async fn fetch_search(uri: &str, state: Arc<AppState>) -> serde_json::Value {
    let app = build_router(state);
    let res = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "expected 200 for {uri}");
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn write_plan_dated(
    dir: &Path,
    date: &str,
    slug_tail: &str,
    title: &str,
    body: &str,
) -> std::path::PathBuf {
    let filename = format!("{date}-{slug_tail}.md");
    let content = format!("---\ntitle: \"{title}\"\n---\n{body}\n");
    let path = dir.join(&filename);
    std::fs::write(&path, content).unwrap();
    path
}

fn write_decision(
    dir: &Path,
    adr_num: &str,
    slug_tail: &str,
    title: &str,
    body: &str,
) -> std::path::PathBuf {
    let filename = format!("ADR-{adr_num}-{slug_tail}.md");
    let content = format!("---\ntitle: \"{title}\"\n---\n{body}\n");
    let path = dir.join(&filename);
    std::fs::write(&path, content).unwrap();
    path
}

#[tokio::test]
async fn empty_q_returns_200_with_empty_results() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();

    let v = fetch_search("/api/search?q=", state.clone()).await;
    assert_eq!(v["results"].as_array().unwrap().len(), 0);

    let v = fetch_search("/api/search", state).await;
    assert_eq!(v["results"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn returns_matches_across_library_doc_types() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();

    let v = fetch_search("/api/search?q=foo", state).await;
    let results = v["results"].as_array().unwrap();
    let doc_types: Vec<&str> = results
        .iter()
        .map(|r| r["docType"].as_str().unwrap())
        .collect();
    assert!(doc_types.contains(&"decisions"), "got {doc_types:?}");
    assert!(doc_types.contains(&"plans"), "got {doc_types:?}");
}

#[tokio::test]
async fn excludes_templates_by_indexer_structure() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let templates_dir = tmp.path().join("meta/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(
        templates_dir.join("foo-template.md"),
        "---\ntitle: \"Foo template\"\n---\n",
    )
    .unwrap();

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();

    let snapshot = state.indexer.all().await;
    assert!(
        snapshot
            .iter()
            .all(|e| e.r#type
                != accelerator_visualiser::docs::DocTypeKey::Templates),
        "Indexer::all() must not yield Templates entries",
    );

    let v = fetch_search("/api/search?q=foo", state).await;
    let results = v["results"].as_array().unwrap();
    for r in results {
        assert_ne!(r["docType"].as_str(), Some("templates"));
    }
}

#[tokio::test]
async fn case_insensitive_matching() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();

    for q in ["foo", "FOO", "Foo"] {
        let uri = format!("/api/search?q={q}");
        let v = fetch_search(&uri, state.clone()).await;
        let results = v["results"].as_array().unwrap();
        assert!(!results.is_empty(), "no results for q={q}");
    }
}

#[tokio::test]
async fn bucket_1_exact_slug_first() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let plans = tmp.path().join("meta/plans");
    // Entry A: slug "alpha" matches exactly; older mtime.
    let p1 = write_plan_dated(
        &plans,
        "2026-04-01",
        "alpha",
        "Some unrelated title",
        "",
    );
    common::set_mtime_ms(&p1, 1_000).unwrap();
    // Entry B: title prefix-matches "alpha"; newer mtime.
    let p2 = write_plan_dated(
        &plans,
        "2026-04-02",
        "beta",
        "Alpha plan with longer title",
        "",
    );
    common::set_mtime_ms(&p2, 9_000_000).unwrap();

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=alpha", state).await;
    let results = v["results"].as_array().unwrap();
    assert!(results.len() >= 2, "results: {results:?}");
    assert_eq!(results[0]["slug"].as_str(), Some("alpha"));
}

#[tokio::test]
async fn bucket_2_prefix_before_interior() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let plans = tmp.path().join("meta/plans");
    let p1 =
        write_plan_dated(&plans, "2026-04-01", "plan-a", "Banana split", "");
    common::set_mtime_ms(&p1, 1_000).unwrap();
    let p2 = write_plan_dated(
        &plans,
        "2026-04-02",
        "plan-b",
        "Has banana inside",
        "",
    );
    common::set_mtime_ms(&p2, 9_000_000).unwrap();

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=banana", state).await;
    let results = v["results"].as_array().unwrap();
    assert!(results.len() >= 2);
    assert_eq!(results[0]["title"].as_str(), Some("Banana split"));
    assert_eq!(results[1]["title"].as_str(), Some("Has banana inside"));
}

#[tokio::test]
async fn bucket_3_title_slug_before_bucket_4_body_preview() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let plans = tmp.path().join("meta/plans");
    let p1 = write_plan_dated(
        &plans,
        "2026-04-01",
        "plan-x",
        "Has zebra inside it",
        "",
    );
    common::set_mtime_ms(&p1, 1_000).unwrap();
    let p2 = write_plan_dated(
        &plans,
        "2026-04-02",
        "plan-y",
        "Totally different",
        "Mentions zebra in the body",
    );
    common::set_mtime_ms(&p2, 9_000_000).unwrap();

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=zebra", state).await;
    let results = v["results"].as_array().unwrap();
    assert!(results.len() >= 2);
    assert_eq!(results[0]["title"].as_str(), Some("Has zebra inside it"));
    assert_eq!(results[1]["title"].as_str(), Some("Totally different"));
}

#[tokio::test]
async fn mtime_desc_within_bucket() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let plans = tmp.path().join("meta/plans");
    let p1 =
        write_plan_dated(&plans, "2026-04-01", "old", "Carrot top old", "");
    common::set_mtime_ms(&p1, 1_000).unwrap();
    let p2 =
        write_plan_dated(&plans, "2026-04-02", "new", "Carrot top new", "");
    common::set_mtime_ms(&p2, 9_000_000).unwrap();

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=carrot", state).await;
    let results = v["results"].as_array().unwrap();
    assert!(results.len() >= 2);
    assert_eq!(results[0]["title"].as_str(), Some("Carrot top new"));
    assert_eq!(results[1]["title"].as_str(), Some("Carrot top old"));
}

#[tokio::test]
async fn path_asc_breaks_mtime_ties() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let plans = tmp.path().join("meta/plans");
    // Same date, different tails — rel_path lexicographic tiebreak.
    let p1 =
        write_plan_dated(&plans, "2026-04-01", "zzz-plan", "Date pudding", "");
    common::set_mtime_ms(&p1, 5_000_000).unwrap();
    let p2 =
        write_plan_dated(&plans, "2026-04-01", "aaa-plan", "Date pudding", "");
    common::set_mtime_ms(&p2, 5_000_000).unwrap();

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=date", state).await;
    let results = v["results"].as_array().unwrap();
    assert!(results.len() >= 2);
    // aaa-plan should come before zzz-plan (lexicographic path ascending).
    assert_eq!(results[0]["slug"].as_str(), Some("aaa-plan"));
    assert_eq!(results[1]["slug"].as_str(), Some("zzz-plan"));
}

#[tokio::test]
async fn slugless_entries_filtered() {
    // Verify every returned result has a non-null slug. With current
    // indexer behaviour, every enumerated entry has a derived slug,
    // but the typed `project` constructor guards against regressions.
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=foo", state).await;
    let results = v["results"].as_array().unwrap();
    for r in results {
        let slug = r["slug"].as_str();
        assert!(slug.is_some(), "every result must have a non-null slug");
        assert!(!slug.unwrap().is_empty());
    }
}

#[tokio::test]
async fn does_not_search_path_or_relpath() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    // Query "qzx" — chosen so it doesn't appear in any seeded fixture's
    // title, slug, or body, but the tmp path always contains "meta" (not "qzx").
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=qzx", state).await;
    let results = v["results"].as_array().unwrap();
    assert_eq!(
        results.len(),
        0,
        "qzx must not match anything; got {results:?}"
    );
}

#[tokio::test]
async fn body_preview_substring_match() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let plans = tmp.path().join("meta/plans");
    write_plan_dated(
        &plans,
        "2026-04-01",
        "narnia",
        "Unrelated",
        "body mentions kumquat fruit",
    );
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=kumquat", state).await;
    let results = v["results"].as_array().unwrap();
    assert!(
        results.iter().any(|r| r["slug"].as_str() == Some("narnia")),
        "expected kumquat body match, got {results:?}"
    );
}

#[tokio::test]
async fn whitespace_only_q_returns_200_with_empty_results() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=%20%20%20", state).await;
    assert_eq!(v["results"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn over_length_q_returns_200_with_empty_results() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let plans = tmp.path().join("meta/plans");
    write_plan_dated(&plans, "2026-04-01", "match", "AAAAAAA matchable", "");
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let oversized: String = "a".repeat(129);
    let uri = format!("/api/search?q={oversized}");
    let v = fetch_search(&uri, state).await;
    assert_eq!(v["results"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn wire_field_is_mtime_ms_camelcase() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let plans = tmp.path().join("meta/plans");
    write_plan_dated(&plans, "2026-04-01", "fluffy", "Fluffy something", "");
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=fluffy", state).await;
    let results = v["results"].as_array().unwrap();
    assert!(!results.is_empty());
    let r = &results[0];
    assert!(r.get("mtimeMs").is_some(), "expected camelCase mtimeMs");
    assert!(r.get("mtime_ms").is_none(), "snake_case must not leak");
    assert!(r.get("docType").is_some());
    assert!(r.get("title").is_some());
    assert!(r.get("slug").is_some());
    // `path` must NOT be in the wire payload (Design Decision 6).
    assert!(r.get("path").is_none(), "path must not be in wire payload");
}

#[tokio::test]
async fn non_matching_entries_are_excluded() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let decisions = tmp.path().join("meta/decisions");
    // Positive: title contains "xylo".
    write_decision(&decisions, "0002", "xylo-decision", "Xylo decision", "");
    // Negative: nothing matches "xylo".
    write_decision(
        &decisions,
        "0003",
        "unrelated",
        "Totally different",
        "no body match",
    );
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=xylo", state).await;
    let results = v["results"].as_array().unwrap();
    let slugs: Vec<&str> = results
        .iter()
        .map(|r| r["slug"].as_str().unwrap())
        .collect();
    assert!(slugs.iter().any(|s| s.contains("xylo")));
    assert!(!slugs.iter().any(|s| s.contains("unrelated")));
}

#[tokio::test]
async fn mixed_case_query_and_field_classify_correctly() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let plans = tmp.path().join("meta/plans");
    write_plan_dated(&plans, "2026-04-01", "foo-bar", "Foo Bar Title", "");
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=FoO", state).await;
    let results = v["results"].as_array().unwrap();
    assert!(results
        .iter()
        .any(|r| r["slug"].as_str() == Some("foo-bar")));
}
