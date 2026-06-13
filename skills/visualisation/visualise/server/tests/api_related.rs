use std::collections::HashMap;
use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::clusters::{compute_clusters, ClusterContext};
use accelerator_visualiser::config::{Config, TemplateTiers};
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

async fn build_app(cfg: Config) -> (Arc<AppState>, axum::Router) {
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state.clone());
    (state, app)
}

async fn json_body(res: axum::response::Response) -> serde_json::Value {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

/// Build a Config rooted at `tmp` with only the doc-type paths the
/// caller cares about. Templates point at empty placeholders so the
/// resolver builds without panicking.
fn cfg_with_only(
    tmp: &std::path::Path,
    doc_paths: HashMap<String, std::path::PathBuf>,
) -> Config {
    let tpl_dir = tmp.join("plugin-templates");
    std::fs::create_dir_all(&tpl_dir).unwrap();
    let mut templates = HashMap::new();
    for name in ["adr", "plan", "research", "validation", "pr-description"] {
        let pd = tpl_dir.join(format!("{name}.md"));
        std::fs::write(&pd, format!("# {name} default\n")).unwrap();
        templates.insert(
            name.to_string(),
            TemplateTiers {
                config_override: None,
                user_override: tmp.join(format!("meta/templates/{name}.md")),
                plugin_default: pd,
                config_override_source: None,
            },
        );
    }
    let tmp_dir = tmp.join("meta/tmp/visualiser");
    std::fs::create_dir_all(&tmp_dir).unwrap();
    Config {
        plugin_root: tmp.to_path_buf(),
        plugin_version: "test".into(),
        project_root: tmp.to_path_buf(),
        tmp_path: tmp_dir,
        host: "127.0.0.1".into(),
        owner_pid: 0,
        owner_start_time: None,
        log_path: tmp.join("server.log"),
        doc_paths,
        templates,
        work_item: None,
        kanban_columns: None,
        idle_timeout: None,
        editor: None,
        editor_project: None,
    }
}

// ── Step 2.1 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_returns_404_for_unknown_path() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let (_state, app) = build_app(cfg).await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/plans/does-not-exist.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body = json_body(res).await;
    assert!(body.get("error").is_some(), "404 carries JSON error body");
}

// ── Step 2.2 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_returns_403_for_path_escape() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let (_state, app) = build_app(cfg).await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/../../etc/passwd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

// ── Step 2.3 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_for_plan_with_no_relations() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(
        plans.join("2026-04-18-isolated.md"),
        "---\ntitle: Isolated\n---\nbody\n",
    )
    .unwrap();
    let mut paths = HashMap::new();
    paths.insert("plans".into(), plans);
    let cfg = cfg_with_only(tmp.path(), paths);
    let (_state, app) = build_app(cfg).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/plans/2026-04-18-isolated.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    assert_eq!(body["inferredCluster"].as_array().unwrap().len(), 0);
    assert_eq!(body["declaredOutbound"].as_array().unwrap().len(), 0);
    assert_eq!(body["declaredInbound"].as_array().unwrap().len(), 0);
}

// ── Step 2.4 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_includes_slug_cluster_siblings() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    let work = tmp.path().join("meta/work");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(plans.join("2026-04-18-foo.md"), "---\ntitle: P\n---\n")
        .unwrap();
    std::fs::write(work.join("0001-foo.md"), "---\ntitle: T\n---\n").unwrap();
    let mut paths = HashMap::new();
    paths.insert("plans".into(), plans);
    paths.insert("work".into(), work);
    let cfg = cfg_with_only(tmp.path(), paths);
    let (_state, app) = build_app(cfg).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/plans/2026-04-18-foo.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let inferred = body["inferredCluster"].as_array().unwrap();
    assert_eq!(inferred.len(), 1);
    assert_eq!(inferred[0]["type"], "work-items");
}

// ── Step 2.5 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_excludes_self_from_inferred() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(plans.join("2026-04-18-foo.md"), "---\ntitle: P1\n---\n")
        .unwrap();
    std::fs::write(
        plans.join("2026-04-18-foo-extra.md"),
        "---\ntitle: P2\n---\n",
    )
    .unwrap();
    let mut paths = HashMap::new();
    paths.insert("plans".into(), plans);
    let cfg = cfg_with_only(tmp.path(), paths);
    let (_state, app) = build_app(cfg).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/plans/2026-04-18-foo.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let inferred = body["inferredCluster"].as_array().unwrap();
    // Self entry must never echo back.
    for e in inferred {
        assert_ne!(
            e["relPath"], "meta/plans/2026-04-18-foo.md",
            "self should never appear in inferred cluster",
        );
    }
}

// ── Step 2.6 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_returns_declared_outbound_for_review() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let (_state, app) = build_app(cfg).await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/reviews/plans/2026-04-18-foo-review-1.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let outbound = body["declaredOutbound"].as_array().unwrap();
    assert_eq!(
        outbound.len(),
        1,
        "review's target plan must appear in outbound"
    );
    assert_eq!(outbound[0]["relPath"], "meta/plans/2026-04-18-foo.md");
    assert_eq!(body["declaredInbound"].as_array().unwrap().len(), 0);
}

// ── Step 2.7 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_returns_declared_inbound_for_target_plan() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let (_state, app) = build_app(cfg).await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/plans/2026-04-18-foo.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let inbound = body["declaredInbound"].as_array().unwrap();
    assert_eq!(inbound.len(), 1, "target plan must list inbound review");
    assert_eq!(
        inbound[0]["relPath"],
        "meta/reviews/plans/2026-04-18-foo-review-1.md"
    );
    assert_eq!(body["declaredOutbound"].as_array().unwrap().len(), 0);
}

// ── Step 2.8 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_returns_empty_outbound_when_target_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    let reviews = tmp.path().join("meta/reviews/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&reviews).unwrap();
    // Note: target plan does NOT exist
    std::fs::write(
        reviews.join("2026-04-18-orphan-review-1.md"),
        "---\ntarget: \"meta/plans/never-created.md\"\n---\n",
    )
    .unwrap();
    let mut paths = HashMap::new();
    paths.insert("plans".into(), plans);
    paths.insert("review_plans".into(), reviews);
    let cfg = cfg_with_only(tmp.path(), paths);
    let (_state, app) = build_app(cfg).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/reviews/plans/2026-04-18-orphan-review-1.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    assert_eq!(
        body["declaredOutbound"].as_array().unwrap().len(),
        0,
        "missing target resolves to empty outbound, not error",
    );
}

// ── Step 2.9 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_returns_404_for_template_path() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let (_state, app) = build_app(cfg).await;
    // Template paths are not addressable through this endpoint —
    // Indexer::get returns None for any path under the templates root,
    // because the indexer skips DocTypeKey::Templates on rescan.
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/templates/adr.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ── Step 2.10 ──────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_response_uses_camelcase_field_names() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let (_state, app) = build_app(cfg).await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/plans/2026-04-18-foo.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let obj = body.as_object().unwrap();
    let keys: std::collections::BTreeSet<&str> =
        obj.keys().map(String::as_str).collect();
    let expected: std::collections::BTreeSet<&str> =
        ["inferredCluster", "declaredOutbound", "declaredInbound"]
            .iter()
            .copied()
            .collect();
    assert_eq!(keys, expected, "wire-format key names");
}

// ── Step 2.11 ──────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_recovers_after_target_creation() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    let reviews = tmp.path().join("meta/reviews/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&reviews).unwrap();
    std::fs::write(
        reviews.join("2026-04-18-foo-review-1.md"),
        "---\ntarget: \"meta/plans/2026-04-18-foo.md\"\n---\n",
    )
    .unwrap();
    let mut paths = HashMap::new();
    paths.insert("plans".into(), plans.clone());
    paths.insert("review_plans".into(), reviews);
    let cfg = cfg_with_only(tmp.path(), paths);
    let (state, app) = build_app(cfg).await;

    // Initially: target plan does not exist → outbound is empty.
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/reviews/plans/2026-04-18-foo-review-1.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(res).await;
    assert_eq!(body["declaredOutbound"].as_array().unwrap().len(), 0);

    // Create the target plan and rescan.
    std::fs::write(plans.join("2026-04-18-foo.md"), "---\ntitle: Foo\n---\n")
        .unwrap();
    state.indexer.rescan().await.unwrap();
    {
        let snapshot = state.indexer.all().await;
        let wbi = state.indexer.work_item_by_id_snapshot().await;
        let pbi = state.indexer.plans_by_id_snapshot().await;
        let ctx = ClusterContext::from_entries(
            &snapshot,
            &wbi,
            &pbi,
            state.indexer.project_root(),
            state.indexer.work_item_cfg(),
        );
        *state.clusters.write().await = compute_clusters(&snapshot, &ctx);
    }

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/reviews/plans/2026-04-18-foo-review-1.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(res).await;
    assert_eq!(
        body["declaredOutbound"].as_array().unwrap().len(),
        1,
        "outbound resolves once the target file exists",
    );
}

// ── Step 2.12 ──────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_dedupes_overlap_in_favor_of_declared() {
    let tmp = tempfile::tempdir().unwrap();
    // The seeded fixture has the review's target plan sharing slug "foo"
    // with the review — exact overlap scenario.
    let cfg = common::seeded_cfg(tmp.path());
    let (_state, app) = build_app(cfg).await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/reviews/plans/2026-04-18-foo-review-1.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let outbound = body["declaredOutbound"].as_array().unwrap();
    let inferred = body["inferredCluster"].as_array().unwrap();
    assert!(
        outbound
            .iter()
            .any(|e| e["relPath"] == "meta/plans/2026-04-18-foo.md"),
        "target plan must appear in outbound",
    );
    for e in inferred {
        assert_ne!(
            e["relPath"], "meta/plans/2026-04-18-foo.md",
            "target plan must NOT appear in inferred — dedup contract",
        );
    }
}

// ── Step 2.13 ──────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_returns_empty_outbound_after_target_deletion() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let (state, app) = build_app(cfg).await;

    // Initially outbound has the plan.
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/reviews/plans/2026-04-18-foo-review-1.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(res).await;
    assert_eq!(body["declaredOutbound"].as_array().unwrap().len(), 1);

    // Delete the target plan.
    let plan_path = tmp.path().join("meta/plans/2026-04-18-foo.md");
    std::fs::remove_file(&plan_path).unwrap();
    state.indexer.refresh_one(&plan_path).await.unwrap();

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/reviews/plans/2026-04-18-foo-review-1.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(res).await;
    assert_eq!(
        body["declaredOutbound"].as_array().unwrap().len(),
        0,
        "outbound resolves through Indexer::get; deleted plan is None",
    );
}

// ── Step 7.1 ───────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_returns_multiple_inbound_reviews() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    let reviews = tmp.path().join("meta/reviews/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::create_dir_all(&reviews).unwrap();
    std::fs::write(
        plans.join("2026-01-01-first-plan.md"),
        "---\ntitle: First\n---\n",
    )
    .unwrap();
    std::fs::write(
        reviews.join("2026-01-01-first-plan-review-1.md"),
        "---\ntarget: \"meta/plans/2026-01-01-first-plan.md\"\n---\n",
    )
    .unwrap();
    std::fs::write(
        reviews.join("2026-01-01-first-plan-review-2.md"),
        "---\ntarget: \"meta/plans/2026-01-01-first-plan.md\"\n---\n",
    )
    .unwrap();
    let mut paths = HashMap::new();
    paths.insert("plans".into(), plans);
    paths.insert("review_plans".into(), reviews);
    let cfg = cfg_with_only(tmp.path(), paths);
    let (_state, app) = build_app(cfg).await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/plans/2026-01-01-first-plan.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let inbound = body["declaredInbound"].as_array().unwrap();
    assert_eq!(
        inbound.len(),
        2,
        "both reviews must appear in declaredInbound"
    );
}

// ── linkedCount: agreement with /api/related sum ───────────────────────────
#[tokio::test]
async fn linked_count_equals_sum_of_related_lists_for_clustered_plan() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let (_state, app) = build_app(cfg).await;

    // Fetch /api/related for the seeded plan to compute the sum.
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/plans/2026-04-18-foo.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let sum = body["inferredCluster"].as_array().unwrap().len()
        + body["declaredOutbound"].as_array().unwrap().len()
        + body["declaredInbound"].as_array().unwrap().len();
    assert!(sum > 0, "seeded fixture has at least one cross-link");

    // Fetch /api/docs?type=plans and find the same entry.
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=plans")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(res).await;
    let plan = body["docs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["relPath"] == "meta/plans/2026-04-18-foo.md")
        .expect("seeded plan must appear in /api/docs");
    assert_eq!(
        plan["linkedCount"].as_u64().unwrap() as usize,
        sum,
        "entry.linkedCount must equal the related endpoint's three-list sum",
    );
}

#[tokio::test]
async fn linked_count_reflects_inbound_merge_dedup() {
    // Single target appearing in both reviews_by_target and
    // work_item_refs_by_id deduplicates to 1, so linkedCount == 1
    // (not 2). The seeded fixture has reviews_by_target alone, so we
    // construct a minimal scenario: a work-item plus a review that
    // targets the work-item AND references the same work-item via
    // work_item_id frontmatter.
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("meta/work");
    let reviews = tmp.path().join("meta/reviews/work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&reviews).unwrap();
    std::fs::write(work.join("0001-target.md"), "---\ntitle: T\n---\n")
        .unwrap();
    // Review file targets the work-item AND declares work_item_id pointing
    // back at the same work-item. Both inbound channels reach the same
    // review path → dedup must collapse to 1.
    std::fs::write(
        reviews.join("0001-target-review-1.md"),
        "---\ntarget: \"meta/work/0001-target.md\"\nwork_item_id: \"0001\"\n---\n",
    )
    .unwrap();
    let mut paths = HashMap::new();
    paths.insert("work".into(), work);
    paths.insert("review_work".into(), reviews);
    let cfg = cfg_with_only(tmp.path(), paths);
    let (_state, app) = build_app(cfg).await;

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/work/0001-target.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(res).await;
    let inbound = body["declaredInbound"].as_array().unwrap();
    assert_eq!(
        inbound.len(),
        1,
        "inbound merge must dedupe duplicate sources"
    );

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=work-items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(res).await;
    let work_item = body["docs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["relPath"] == "meta/work/0001-target.md")
        .expect("work item must appear in /api/docs");
    assert_eq!(
        work_item["linkedCount"].as_u64().unwrap(),
        1,
        "linkedCount mirrors the deduped count, not 2",
    );
}

// ── RCA inbound linkage (work item 0110, AC #5) ─────────────────────────────
#[tokio::test]
async fn related_endpoint_surfaces_rca_linked_to_a_work_item() {
    // An RCA whose frontmatter links to a work item (via `parent:`) appears in
    // that work item's declaredInbound through work_item_refs_by_id, with
    // docType `root-cause-analyses` — the server half of AC #5. `parent:` uses
    // the bare-numeric form because `canonicalise_one_id` only resolves
    // bare-numeric / project-prefixed refs (not the `work-item:` typed prefix)
    // for the aggregating `parent:`/`related:` keys.
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("meta/work");
    let issues = tmp.path().join("meta/research/issues");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&issues).unwrap();
    std::fs::write(work.join("0001-target.md"), "---\ntitle: T\n---\n")
        .unwrap();
    std::fs::write(
        issues.join("2026-06-10-example-rca.md"),
        "---\ntitle: \"Example RCA\"\ntype: issue-research\nstatus: resolved\nparent: \"0001\"\n---\n# body\n",
    )
    .unwrap();
    let mut paths = HashMap::new();
    paths.insert("work".into(), work);
    paths.insert("research_issues".into(), issues);
    let cfg = cfg_with_only(tmp.path(), paths);
    let (_state, app) = build_app(cfg).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/meta/work/0001-target.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let inbound = body["declaredInbound"].as_array().unwrap();
    assert_eq!(inbound.len(), 1, "the linking RCA must appear inbound");
    assert_eq!(inbound[0]["type"], "root-cause-analyses");
    assert_eq!(
        inbound[0]["relPath"],
        "meta/research/issues/2026-06-10-example-rca.md"
    );
}

// ── Step 2.14 ──────────────────────────────────────────────────────────────
#[tokio::test]
async fn related_endpoint_validates_decoded_path_segments() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let (_state, app) = build_app(cfg).await;
    // foo%2F..%2Fbar — once decoded becomes foo/../bar; the per-segment
    // validator must catch the `..` segment after decoding.
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/related/foo%2F..%2Fbar")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}
