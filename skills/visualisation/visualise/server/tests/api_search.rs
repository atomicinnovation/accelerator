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
async fn returns_rca_for_a_title_query() {
    // An RCA under research/issues is auto-included in search (non-virtual,
    // config_path_key mapped, non-null slug), labelled/routed as an RCA — the
    // server half of AC #6 (work item 0110).
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let issues = tmp.path().join("meta/research/issues");
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(
        issues.join("2026-06-10-kryptonite-rca.md"),
        "---\ntitle: \"Kryptonite RCA\"\ntype: issue-research\nstatus: resolved\n---\n# body\n",
    )
    .unwrap();
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=kryptonite", state).await;
    let results = v["results"].as_array().unwrap();
    let rca = results
        .iter()
        .find(|r| r["docType"].as_str() == Some("root-cause-analyses"))
        .expect("RCA must appear in search results");
    assert_eq!(rca["slug"].as_str(), Some("kryptonite-rca"));
    assert!(!rca["slug"].as_str().unwrap().is_empty());
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

// --- work_item_id matching (work item 0125) ---------------------------------

/// Write a work-item fixture. `filename_stem` is the file name without `.md`
/// (must begin with `<digits>-` so a slug derives), and `id` is the optional
/// frontmatter `id:` value — `None` omits the key so the entry's
/// `work_item_id` is `None`. Filename and frontmatter id are decoupled so the
/// project-code tests can pin a specific `rel_path` ordering independent of the
/// stored id.
fn write_work_item(
    dir: &Path,
    filename_stem: &str,
    id: Option<&str>,
    title: &str,
    body: &str,
) -> std::path::PathBuf {
    let id_line = id.map(|v| format!("id: \"{v}\"\n")).unwrap_or_default();
    let content = format!("---\n{id_line}title: \"{title}\"\n---\n{body}\n");
    let path = dir.join(format!("{filename_stem}.md"));
    std::fs::write(&path, content).unwrap();
    path
}

fn result_slugs(v: &serde_json::Value) -> Vec<String> {
    v["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["slug"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn slug_pos(v: &serde_json::Value, slug: &str) -> Option<usize> {
    result_slugs(v).iter().position(|s| s == slug)
}

// Item 1: guards the load-bearing assumption (inverse of the
// `seeded_cfg_with_work_items` gotcha): a fixture carrying `id:` populates
// `work_item_id`. If this regresses, it names the real cause rather than
// letting the search assertions fail opaquely.
#[tokio::test]
async fn work_item_id_fixture_is_populated() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    write_work_item(
        &work,
        "0042-login-form",
        Some("0042"),
        "Login form",
        "# body",
    );

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let snapshot = state.indexer.all().await;
    let entry = snapshot
        .iter()
        .find(|e| e.slug.as_deref() == Some("login-form"))
        .expect("fixture entry must be indexed");
    assert_eq!(entry.work_item_id.as_deref(), Some("0042"));
}

// Item 2: an exact id match outranks a body hit on the same query.
#[tokio::test]
async fn work_item_id_exact_ranks_above_body() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    write_work_item(
        &work,
        "0042-login-form",
        Some("0042"),
        "Login form",
        "# body",
    );
    // Competitor: body contains 0042 -> Body bucket only.
    write_work_item(
        &work,
        "0099-other",
        Some("0099"),
        "Other thing",
        "mentions 0042 inline",
    );

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=0042", state).await;
    assert_eq!(
        v["results"][0]["slug"].as_str(),
        Some("login-form"),
        "exact id must outrank a body hit; got {:?}",
        result_slugs(&v)
    );
}

// Item 3: an unpadded numeric query (`42`) reduces to the same numeric key as
// `0042` and ranks exact (above a body-only competitor).
#[tokio::test]
async fn unpadded_numeric_query_is_exact() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    write_work_item(
        &work,
        "0042-login-form",
        Some("0042"),
        "Login form",
        "# body",
    );
    write_work_item(
        &work,
        "0099-other",
        Some("0099"),
        "Other thing",
        "ref 42 here",
    );

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=42", state).await;
    assert_eq!(
        v["results"][0]["slug"].as_str(),
        Some("login-form"),
        "unpadded `42` must exact-match `0042`; got {:?}",
        result_slugs(&v)
    );
}

// Item 4: prefix outranks interior, isolating the numeric `id_prefix` branch.
// Query `004` reduces to `4`; target `0042` -> key `42` (prefix-matches `4`),
// competitor `0014` -> key `14` (interior-matches `4`). Mtimes pinned equal so
// the within-bucket `rel_path` tiebreak (competitor `0014-…` sorts first)
// flips the order if `id_prefix` is deleted.
#[tokio::test]
async fn work_item_id_prefix_outranks_interior() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    let target = write_work_item(
        &work,
        "0042-login-form",
        Some("0042"),
        "Login form",
        "# body",
    );
    let competitor = write_work_item(
        &work,
        "0014-zzz",
        Some("0014"),
        "Some thing",
        "# body",
    );
    common::set_mtime_ms(&target, 5_000_000).unwrap();
    common::set_mtime_ms(&competitor, 5_000_000).unwrap();

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=004", state).await;
    let tp = slug_pos(&v, "login-form").expect("target must be returned");
    let cp = slug_pos(&v, "zzz").expect("competitor must be returned");
    assert!(
        tp < cp,
        "prefix (target) must outrank interior (competitor); got {:?}",
        result_slugs(&v)
    );
}

// Item 5: interior is returned but ranks below a prefix competitor, isolating
// the numeric `id_interior` branch. Query `2`; target `0042` -> key `42`
// (interior-matches `2`), competitor `0020` -> key `20` (prefix-matches `2`).
// Deleting `id_interior` drops the target to Body (no body match) so it
// disappears, failing the "target returned" assertion.
#[tokio::test]
async fn work_item_id_interior_outranks_nothing_but_below_prefix() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    write_work_item(
        &work,
        "0042-login-form",
        Some("0042"),
        "Login form",
        "# body",
    );
    write_work_item(&work, "0020-aaa", Some("0020"), "Some thing", "# body");

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=2", state).await;
    let tp = slug_pos(&v, "login-form").expect("target must be returned");
    let cp = slug_pos(&v, "aaa").expect("prefix competitor must be returned");
    assert!(
        cp < tp,
        "prefix competitor must outrank interior target; got {:?}",
        result_slugs(&v)
    );
}

// Item 6 (negative): a numeric query absent from the id's numeric key matches
// nothing.
#[tokio::test]
async fn numeric_query_does_not_match_unrelated_id() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    write_work_item(
        &work,
        "0042-login-form",
        Some("0042"),
        "Login form",
        "# body",
    );

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=0099", state).await;
    assert!(
        slug_pos(&v, "login-form").is_none(),
        "0099 must not match 0042; got {:?}",
        result_slugs(&v)
    );
}

// Item 7 (flood guard): `00` reduces to the empty numeric key and must match no
// id. Titles/bodies are deliberately free of the substring `00`, so this
// asserts specifically the `id_active` empty-key branch (not incidental field
// contents).
#[tokio::test]
async fn all_zero_query_does_not_flood() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    write_work_item(
        &work,
        "0042-login-form",
        Some("0042"),
        "Login form",
        "# body",
    );
    write_work_item(
        &work,
        "0001-todo-item",
        Some("0001"),
        "Todo item",
        "# body",
    );

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=00", state).await;
    assert!(
        slug_pos(&v, "login-form").is_none()
            && slug_pos(&v, "todo-item").is_none(),
        "all-zero query must match no id; got {:?}",
        result_slugs(&v)
    );
}

// Item 8: a `None`-id entry (no `id:` key) is never matched by a number, and
// the query does not panic.
#[tokio::test]
async fn none_id_entry_is_not_matched_by_number() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    write_work_item(
        &work,
        "0042-login-form",
        Some("0042"),
        "Login form",
        "# body",
    );
    // No `id:` key -> work_item_id None; content free of the query digits.
    write_work_item(&work, "0050-plain-item", None, "Plain item", "# body");

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=42", state).await;
    assert!(
        slug_pos(&v, "login-form").is_some(),
        "id-bearing item must match"
    );
    assert!(
        slug_pos(&v, "plain-item").is_none(),
        "None-id entry must not match a number; got {:?}",
        result_slugs(&v)
    );
}

// Item 9: under a project-code config the numeric tail and the full case-folded
// key both match, config-independently. Asserts the wiring (work_item_id ==
// `ENG-0042`) first so a config->work_item_id regression is named explicitly.
#[tokio::test]
async fn project_code_config_matches_numeric_and_full_key() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg_project_code(tmp.path(), "ENG");
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    // Filename stays 0042-…; normalisation prepends the code -> ENG-0042.
    write_work_item(
        &work,
        "0042-login-form",
        Some("0042"),
        "Login form",
        "# body",
    );

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();

    let snapshot = state.indexer.all().await;
    let entry = snapshot
        .iter()
        .find(|e| e.slug.as_deref() == Some("login-form"))
        .expect("fixture entry must be indexed");
    assert_eq!(
        entry.work_item_id.as_deref(),
        Some("ENG-0042"),
        "project code must be wired into work_item_id"
    );

    for q in ["42", "eng-0042", "0042"] {
        let uri = format!("/api/search?q={q}");
        let v = fetch_search(&uri, state.clone()).await;
        assert!(
            slug_pos(&v, "login-form").is_some(),
            "q={q} must surface the item; got {:?}",
            result_slugs(&v)
        );
    }
}

// Item 9 sibling: a foreign-prefixed id distinct from the configured code
// (`OPS-7` under an `ENG` config) is stored verbatim by normalise_id and is
// reachable by both its numeric tail and its full key.
#[tokio::test]
async fn foreign_prefixed_id_matches_under_project_code() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg_project_code(tmp.path(), "ENG");
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    write_work_item(
        &work,
        "0007-ops-task",
        Some("OPS-7"),
        "Ops task",
        "# body",
    );

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();

    let snapshot = state.indexer.all().await;
    let entry = snapshot
        .iter()
        .find(|e| e.slug.as_deref() == Some("ops-task"))
        .expect("fixture entry must be indexed");
    assert_eq!(entry.work_item_id.as_deref(), Some("OPS-7"));

    for q in ["7", "ops-7"] {
        let uri = format!("/api/search?q={q}");
        let v = fetch_search(&uri, state.clone()).await;
        assert!(
            slug_pos(&v, "ops-task").is_some(),
            "q={q} must surface OPS-7; got {:?}",
            result_slugs(&v)
        );
    }
}

// Item 10: the result set is capped at MAX_RESULTS, applied after bucket
// sorting so the lone exact hit survives at the top. (50 == api::search::
// MAX_RESULTS, kept in sync; the value is not a wire contract.)
#[tokio::test]
async fn results_are_capped_and_preserve_top_hits() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    // Exact target: numeric_key("0007") == "7".
    write_work_item(
        &work,
        "0007-exact-target",
        Some("0007"),
        "Exact target",
        "# body",
    );
    // 55 interior-only entries: ids 1700..=1754 -> keys "17XX", each contains
    // `7` interiorly (never as a prefix, never exact), so none perturbs the top.
    for i in 0..55u32 {
        let id = (1700 + i).to_string();
        let stem = format!("{id}-flood-item");
        write_work_item(&work, &stem, Some(&id), "Flood item", "# body");
    }

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=7", state).await;
    let results = v["results"].as_array().unwrap();
    assert_eq!(
        results.len(),
        50,
        "result set must be capped at MAX_RESULTS"
    );
    assert_eq!(
        results[0]["slug"].as_str(),
        Some("exact-target"),
        "exact hit must survive truncation at the top"
    );
}

// Item 11 (non-numeric prefix branch): under a project-code config, `eng-00`
// prefix-matches `ENG-0042`'s full key. The competitor interior-matches via a
// *title* (`releng-00x`), not its id (`ENG-7777`). Mtimes pinned equal and the
// competitor's rel_path (`0001-…`) sorts first, so deleting the non-numeric
// `id_prefix` branch (which leaves the target in Interior via id_interior)
// flips the order.
#[tokio::test]
async fn non_numeric_id_prefix_outranks_interior() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg_project_code(tmp.path(), "ENG");
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    let target = write_work_item(
        &work,
        "0042-target",
        Some("0042"),
        "Target item",
        "# body",
    );
    // Filename 0001-… (sorts first); id ENG-7777 (no eng-00 match); title
    // interior-matches eng-00.
    let competitor = write_work_item(
        &work,
        "0001-comp",
        Some("ENG-7777"),
        "releng-00x competitor",
        "# body",
    );
    common::set_mtime_ms(&target, 5_000_000).unwrap();
    common::set_mtime_ms(&competitor, 5_000_000).unwrap();

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=eng-00", state).await;
    let tp = slug_pos(&v, "target").expect("target must be returned");
    let cp = slug_pos(&v, "comp").expect("competitor must be returned");
    assert!(
        tp < cp,
        "non-numeric prefix (target) must outrank interior (competitor); got {:?}",
        result_slugs(&v)
    );
}

// Item 12 (non-numeric interior branch): `ng-00` interior-matches `ENG-0042`
// but is not a prefix of it. The competitor's Prefix placement comes from its
// *title* (`ng-00 …`), not its id. Cross-bucket (Interior < Prefix), robust to
// mtime. Deleting the non-numeric `id_interior` branch drops the target to Body
// (no body match), so it disappears.
#[tokio::test]
async fn non_numeric_id_interior_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg_project_code(tmp.path(), "ENG");
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    write_work_item(
        &work,
        "0042-target",
        Some("0042"),
        "Target item",
        "# body",
    );
    write_work_item(
        &work,
        "0001-comp",
        Some("ENG-7777"),
        "ng-00 competitor",
        "# body",
    );

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let v = fetch_search("/api/search?q=ng-00", state).await;
    let tp = slug_pos(&v, "target").expect("target must be returned");
    let cp = slug_pos(&v, "comp").expect("prefix competitor must be returned");
    assert!(
        cp < tp,
        "prefix competitor must outrank interior target; got {:?}",
        result_slugs(&v)
    );
}
