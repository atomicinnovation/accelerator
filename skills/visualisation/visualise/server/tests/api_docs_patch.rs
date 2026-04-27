use std::sync::Arc;
use std::time::Duration;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use accelerator_visualiser::sse_hub::SsePayload;
use accelerator_visualiser::watcher;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use tokio::sync::RwLock;
use tower::ServiceExt;

mod common;

const TICKET_PATH: &str = "meta/tickets/0001-todo-fixture.md";
const PLAN_PATH: &str = "meta/plans/2026-04-18-foo.md";

async fn setup(tmp: &std::path::Path) -> Arc<AppState> {
    let cfg = common::seeded_cfg_with_tickets(tmp);
    let activity = Arc::new(Activity::new());
    AppState::build(cfg, activity).await.unwrap()
}

async fn fetch_etag(state: Arc<AppState>, rel: &str) -> String {
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/docs/{rel}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "GET failed while fetching etag"
    );
    res.headers()
        .get(header::ETAG)
        .expect("ETag header missing")
        .to_str()
        .unwrap()
        .to_string()
}

fn patch_req(rel: &str, if_match: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/docs/{rel}/frontmatter"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, if_match)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn patch_req_no_if_match(rel: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/docs/{rel}/frontmatter"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn read_body(res: axum::response::Response) -> serde_json::Value {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

// ── Step 3.1 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_succeeds_with_correct_if_match_returns_204_and_new_etag() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let etag = fetch_etag(state.clone(), TICKET_PATH).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            TICKET_PATH,
            &etag,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let new_etag = res
        .headers()
        .get(header::ETAG)
        .expect("ETag header missing on 204")
        .to_str()
        .unwrap()
        .to_string();
    assert_ne!(new_etag, etag, "ETag should change after a real write");

    let on_disk = tokio::fs::read_to_string(tmp.path().join(TICKET_PATH))
        .await
        .unwrap();
    assert!(
        on_disk.contains("status: in-progress"),
        "on-disk file should have status: in-progress, got:\n{on_disk}",
    );
}

// ── Step 3.2 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_broadcasts_doc_changed_with_new_etag() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let etag = fetch_etag(state.clone(), TICKET_PATH).await;
    let mut rx = state.sse_hub.subscribe();

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            TICKET_PATH,
            &etag,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let response_etag = res
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("timed out waiting for SSE event")
        .expect("channel closed");

    match event {
        SsePayload::DocChanged {
            doc_type,
            path,
            etag: Some(broadcast_etag),
        } => {
            assert_eq!(doc_type, accelerator_visualiser::docs::DocTypeKey::Tickets);
            assert_eq!(path, TICKET_PATH);
            assert_eq!(
                format!("\"{}\"", broadcast_etag),
                response_etag,
                "broadcast etag must match response ETag header"
            );
        }
        other => panic!("expected DocChanged with etag, got {other:?}"),
    }
}

// ── Step 3.3a ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_with_stale_if_match_returns_412_when_indexer_not_refreshed() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let original_etag = fetch_etag(state.clone(), TICKET_PATH).await;

    // Out-of-band edit — do NOT call refresh_one; index is now stale.
    tokio::fs::write(
        tmp.path().join(TICKET_PATH),
        "---\ntitle: \"Todo fixture\"\ntype: adr-creation-task\nstatus: done\n---\n# body\n",
    )
    .await
    .unwrap();

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            TICKET_PATH,
            &original_etag,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::PRECONDITION_FAILED);
    let body = read_body(res).await;
    assert!(
        body["currentEtag"].is_string(),
        "412 body must include currentEtag: {body}",
    );
    let current = body["currentEtag"].as_str().unwrap();
    assert_ne!(
        current,
        original_etag.trim_matches('"'),
        "currentEtag must differ from stale etag"
    );
}

// ── Step 3.3b ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_with_stale_if_match_returns_412_when_indexer_refreshed() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let original_etag = fetch_etag(state.clone(), TICKET_PATH).await;
    let abs = tmp.path().join(TICKET_PATH);

    // Out-of-band edit then refresh index.
    tokio::fs::write(
        &abs,
        "---\ntitle: \"Todo fixture\"\ntype: adr-creation-task\nstatus: done\n---\n# body\n",
    )
    .await
    .unwrap();
    let _ = state.indexer.refresh_one(&abs).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            TICKET_PATH,
            &original_etag,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::PRECONDITION_FAILED);
    let body = read_body(res).await;
    assert!(
        body["currentEtag"].is_string(),
        "412 body must include currentEtag: {body}",
    );
}

// ── Step 3.4 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_without_if_match_returns_428() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req_no_if_match(
            TICKET_PATH,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status().as_u16(), 428);
    let body = read_body(res).await;
    assert_eq!(
        body["error"].as_str(),
        Some("if-match-required"),
        "428 body must have error=if-match-required: {body}",
    );
    // 428 must NOT include currentEtag.
    assert!(
        body.get("currentEtag").is_none(),
        "428 must not include currentEtag: {body}",
    );
}

// ── Step 3.4b ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_with_unsupported_if_match_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let app = build_router(state);

    // Wildcard
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/docs/{TICKET_PATH}/frontmatter"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::IF_MATCH, "*")
                .body(Body::from(r#"{"patch":{"status":"in-progress"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "wildcard If-Match");

    // Weak etag
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/docs/{TICKET_PATH}/frontmatter"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::IF_MATCH, "W/\"sha256-abc\"")
                .body(Body::from(r#"{"patch":{"status":"in-progress"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "weak If-Match");

    // Etag list
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/docs/{TICKET_PATH}/frontmatter"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::IF_MATCH, "\"sha256-a\", \"sha256-b\"")
                .body(Body::from(r#"{"patch":{"status":"in-progress"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "etag list If-Match");
}

// ── Step 3.5 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_with_unknown_status_value_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let etag = fetch_etag(state.clone(), TICKET_PATH).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            TICKET_PATH,
            &etag,
            r#"{"patch":{"status":"blocked"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── Step 3.6 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_with_disallowed_field_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let etag = fetch_etag(state.clone(), TICKET_PATH).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            TICKET_PATH,
            &etag,
            r#"{"patch":{"title":"foo"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── Step 3.7 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_with_empty_patch_object_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let etag = fetch_etag(state.clone(), TICKET_PATH).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(TICKET_PATH, &etag, r#"{"patch":{}}"#))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── Step 3.8 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_to_non_ticket_path_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let plan_etag = fetch_etag(state.clone(), PLAN_PATH).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            PLAN_PATH,
            &plan_etag,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── Step 3.9 ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_to_missing_path_returns_404() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            "meta/tickets/9999-ghost.md",
            "\"sha256-doesntmatter\"",
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ── Step 3.10a ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn path_with_dotdot_segment_rejected_at_handler() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;

    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/docs/meta/tickets/../plans/foo.md/frontmatter")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::IF_MATCH, "\"sha256-x\"")
                .body(Body::from(r#"{"patch":{"status":"in-progress"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

// ── Step 3.10b (unix only) ───────────────────────────────────────────────────

#[cfg(unix)]
#[tokio::test]
async fn path_passing_handler_check_but_resolving_outside_writable_roots_rejected_at_driver() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;

    // Create a symlink inside tickets/ pointing outside the writable root.
    let tickets_dir = tmp.path().join("meta/tickets");
    let plans_dir = tmp.path().join("meta/plans");
    let sneaky = tickets_dir.join("sneaky.md");
    std::os::unix::fs::symlink(plans_dir.join("2026-04-18-foo.md"), &sneaky).unwrap();

    let etag = fetch_etag(state.clone(), "meta/tickets/sneaky.md").await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            "meta/tickets/sneaky.md",
            &etag,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();

    // Driver rejects as PathNotWritable → 400
    assert!(
        res.status().is_client_error(),
        "expected client error for symlink escape, got {}",
        res.status()
    );
}

// ── Step 3.10c ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn legitimate_filename_with_dots_is_not_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;

    // PATCH against a path with ".." as a substring (not a segment) must NOT return 403.
    // No such file exists, so we expect 404 — not 403.
    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            "meta/tickets/0001..todo.md",
            "\"sha256-fake\"",
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();

    assert_ne!(
        res.status(),
        StatusCode::FORBIDDEN,
        "filename containing '..' substring must not be rejected as path escape",
    );
    // Either 404 (not found) is expected.
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ── Step 3.11 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_url_without_frontmatter_suffix_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;

    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/docs/{TICKET_PATH}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::IF_MATCH, "\"sha256-x\"")
                .body(Body::from(r#"{"patch":{"status":"in-progress"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = read_body(res).await;
    let msg = body["error"].as_str().unwrap_or("");
    assert!(
        msg.contains("frontmatter"),
        "error message should mention 'frontmatter', got: {msg}",
    );
}

// ── Step 3.12 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_with_invalid_json_body_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(TICKET_PATH, "\"sha256-x\"", "not json at all"))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── Step 3.13 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_request_unaffected_by_patch_method_being_added() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;

    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/docs/{TICKET_PATH}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.contains("text/markdown"),
        "expected markdown content-type, got {ct}"
    );
}

// ── Step 3.14 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn idempotent_patch_with_same_value_returns_204_with_unchanged_etag_and_no_broadcast() {
    let tmp = tempfile::tempdir().unwrap();
    // Use a done-fixture so we can PATCH with status=done (same value).
    let state = setup(tmp.path()).await;
    let done_path = "meta/tickets/0002-done-fixture.md";
    let etag = fetch_etag(state.clone(), done_path).await;
    let mtime_before = tokio::fs::metadata(tmp.path().join(done_path))
        .await
        .unwrap()
        .modified()
        .unwrap();

    let mut rx = state.sse_hub.subscribe();

    // Pass a clone so `state` stays alive and the channel remains open after oneshot.
    let app = build_router(state.clone());
    let res = app
        .oneshot(patch_req(
            done_path,
            &etag,
            r#"{"patch":{"status":"done"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let response_etag = res
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(
        response_etag, etag,
        "idempotent PATCH must return the same ETag"
    );

    let mtime_after = tokio::fs::metadata(tmp.path().join(done_path))
        .await
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        mtime_before, mtime_after,
        "mtime must not change for idempotent PATCH"
    );

    assert!(
        tokio::time::timeout(Duration::from_millis(50), rx.recv())
            .await
            .is_err(),
        "idempotent PATCH must not broadcast any SSE event",
    );
}

// ── Step 3.14b ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_with_unknown_field_in_body_returns_400() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let etag = fetch_etag(state.clone(), TICKET_PATH).await;

    let app = build_router(state);
    let res = app
        .oneshot(patch_req(
            TICKET_PATH,
            &etag,
            r#"{"patch":{"status":"todo","title":"hijack"}}"#,
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── Step 3.14c ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_from_disallowed_origin_returns_403_allowed_origins_succeed() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let etag = fetch_etag(state.clone(), TICKET_PATH).await;
    let app = build_router(state);

    // Foreign origin → 403
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/docs/{TICKET_PATH}/frontmatter"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::IF_MATCH, &etag)
                .header("origin", "https://evil.example")
                .body(Body::from(r#"{"patch":{"status":"in-progress"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "foreign origin must be rejected"
    );

    // No Origin header (curl-style) → succeeds
    let res = app
        .clone()
        .oneshot(patch_req(
            TICKET_PATH,
            &etag,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::NO_CONTENT,
        "no-origin request must succeed"
    );
}

// ── Step 3.15 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_emits_exactly_one_doc_changed_event() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let etag = fetch_etag(state.clone(), TICKET_PATH).await;

    let mut rx = state.sse_hub.subscribe();
    let clusters = Arc::new(RwLock::new(
        accelerator_visualiser::clusters::compute_clusters(&state.indexer.all().await),
    ));

    let app = build_router(state.clone());
    let res = app
        .oneshot(patch_req(
            TICKET_PATH,
            &etag,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Consume the handler-driven broadcast.
    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("timed out waiting for SSE event")
        .expect("channel closed");
    assert!(
        matches!(event, SsePayload::DocChanged { .. }),
        "expected DocChanged, got {event:?}"
    );

    // Now synthesise a watcher event for the same path.
    let canonical = tokio::fs::canonicalize(tmp.path().join(TICKET_PATH))
        .await
        .unwrap();
    watcher::on_path_changed_debounced(
        canonical,
        tmp.path().to_path_buf(),
        state.indexer.clone(),
        clusters,
        state.sse_hub.clone(),
        state.write_coordinator.clone(),
        Duration::ZERO,
        None,
    )
    .await;

    // The watcher must suppress its broadcast — no second event.
    assert!(
        tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .is_err(),
        "watcher must not emit a second broadcast after handler already did",
    );
}

// ── Step 3.16 (unix only) ────────────────────────────────────────────────────

#[cfg(unix)]
#[tokio::test]
async fn patch_dedup_works_when_watcher_event_path_is_non_canonical() {
    let tmp = tempfile::tempdir().unwrap();
    // Create a symlinked tickets directory.
    let real_dir = tmp.path().join("meta/tickets_real");
    let link_dir = tmp.path().join("meta/tickets_link");
    tokio::fs::create_dir_all(&real_dir).await.unwrap();
    tokio::fs::write(
        real_dir.join("0001-todo-fixture.md"),
        "---\ntitle: \"Todo fixture\"\ntype: adr-creation-task\nstatus: todo\n---\n# body\n",
    )
    .await
    .unwrap();
    std::os::unix::fs::symlink(&real_dir, &link_dir).unwrap();

    // Build a cfg pointing the tickets root at the symlinked path.
    let mut cfg = common::seeded_cfg_with_tickets(tmp.path());
    cfg.doc_paths.insert("tickets".into(), link_dir.clone());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();

    let ticket_via_link = "meta/tickets_link/0001-todo-fixture.md";
    let etag = fetch_etag(state.clone(), ticket_via_link).await;
    let mut rx = state.sse_hub.subscribe();
    let clusters = Arc::new(RwLock::new(
        accelerator_visualiser::clusters::compute_clusters(&state.indexer.all().await),
    ));

    let app = build_router(state.clone());
    let res = app
        .oneshot(patch_req(
            ticket_via_link,
            &etag,
            r#"{"patch":{"status":"in-progress"}}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Consume the handler broadcast.
    let _ = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("handler broadcast missing")
        .expect("channel closed");

    // Synthesise watcher event with the non-canonical (symlink) path.
    let symlink_path = link_dir.join("0001-todo-fixture.md");
    watcher::on_path_changed_debounced(
        symlink_path,
        tmp.path().to_path_buf(),
        state.indexer.clone(),
        clusters,
        state.sse_hub.clone(),
        state.write_coordinator.clone(),
        Duration::ZERO,
        None,
    )
    .await;

    // Watcher must suppress — exactly one event total.
    assert!(
        tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .is_err(),
        "watcher must suppress duplicate broadcast even when path is non-canonical",
    );
}

// ── Step 3.17 ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn patch_does_not_register_self_write_on_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let state = setup(tmp.path()).await;
    let done_path = "meta/tickets/0002-done-fixture.md";
    let etag = fetch_etag(state.clone(), done_path).await;

    let app = build_router(state.clone());
    let res = app
        .oneshot(patch_req(
            done_path,
            &etag,
            r#"{"patch":{"status":"done"}}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let canonical = tokio::fs::canonicalize(tmp.path().join(done_path))
        .await
        .unwrap();
    assert!(
        !state.write_coordinator.should_suppress(&canonical),
        "idempotent PATCH must not insert an entry into WriteCoordinator",
    );
}
