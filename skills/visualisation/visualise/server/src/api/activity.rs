//! `GET /api/activity?limit=N` — returns the N most recent file-change
//! events from the in-memory ring buffer (newest-first).

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::activity_feed::{ActivityEvent, CAPACITY};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub(crate) struct LimitParam {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ActivityResponse {
    events: Vec<ActivityEvent>,
}

/// `GET /api/activity?limit=N` — returns up to `min(N, CAPACITY)` recent
/// file-change events, newest-first. Default `limit` is `CAPACITY` (50).
///
/// Contract:
///
/// - `?limit=N` with `0 <= N <= CAPACITY` → returns up to N events.
/// - `?limit=N` with `N > CAPACITY` → silently clamped at `CAPACITY`. The
///   response does NOT signal the clamp. Clients reasoning about response
///   size must not assume `events.len() == requested limit`.
/// - `?limit=foo` (non-numeric) or `?limit=-1` → Axum's `Query` extractor
///   returns 400.
/// - No `?limit` → default `CAPACITY`.
pub(crate) async fn activity(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LimitParam>,
) -> Json<ActivityResponse> {
    let limit = params.limit.unwrap_or(CAPACITY).min(CAPACITY);
    let events = state.activity_feed.recent(limit);
    Json(ActivityResponse { events })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity_feed::ActivityRingBuffer;
    use crate::docs::DocTypeKey;
    use crate::server::AppState;
    use crate::sse_hub::ActionKind;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::{TimeZone, Utc};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt;

    async fn minimal_state() -> Arc<AppState> {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = crate::config::Config {
            plugin_root: tmp.path().to_path_buf(),
            plugin_version: "test".into(),
            project_root: tmp.path().to_path_buf(),
            tmp_path: tmp.path().to_path_buf(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: tmp.path().join("server.log"),
            doc_paths: Default::default(),
            templates: Default::default(),
            work_item: None,
            kanban_columns: None,
        };
        let http_activity = Arc::new(crate::activity::Activity::new());
        AppState::build(cfg, http_activity).await.unwrap()
    }

    fn ev(seconds: i64) -> ActivityEvent {
        ActivityEvent {
            action: ActionKind::Edited,
            doc_type: DocTypeKey::Plans,
            path: format!("meta/plans/t{seconds}.md"),
            timestamp: Utc.timestamp_opt(seconds, 0).unwrap(),
        }
    }

    async fn get(state: Arc<AppState>, uri: &str) -> (StatusCode, Vec<u8>) {
        let app = crate::server::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes().to_vec();
        (status, bytes)
    }

    fn parse(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap()
    }

    #[tokio::test]
    async fn returns_newest_first_capped_at_limit() {
        let state = minimal_state().await;
        let buf: &ActivityRingBuffer = state.activity_feed.as_ref();
        for i in 1..=6_i64 {
            buf.push(ev(i));
        }
        let (status, body) = get(state, "/api/activity?limit=5").await;
        assert_eq!(status, StatusCode::OK);
        let v = parse(&body);
        let events = v["events"].as_array().unwrap();
        assert_eq!(events.len(), 5);
        // Newest first.
        assert_eq!(events[0]["timestamp"].as_str().unwrap(), "1970-01-01T00:00:06Z");
        assert_eq!(events[4]["timestamp"].as_str().unwrap(), "1970-01-01T00:00:02Z");
    }

    #[tokio::test]
    async fn at_capacity_returns_fifty_newest() {
        let state = minimal_state().await;
        let buf: &ActivityRingBuffer = state.activity_feed.as_ref();
        for i in 1..=51_i64 {
            buf.push(ev(i));
        }
        let (status, body) = get(state, "/api/activity?limit=50").await;
        assert_eq!(status, StatusCode::OK);
        let v = parse(&body);
        let events = v["events"].as_array().unwrap();
        assert_eq!(events.len(), 50);
        assert_eq!(events[0]["timestamp"].as_str().unwrap(), "1970-01-01T00:00:51Z");
        assert_eq!(events[49]["timestamp"].as_str().unwrap(), "1970-01-01T00:00:02Z");
    }

    #[tokio::test]
    async fn default_limit_returns_up_to_capacity() {
        let state = minimal_state().await;
        let buf: &ActivityRingBuffer = state.activity_feed.as_ref();
        for i in 1..=10_i64 {
            buf.push(ev(i));
        }
        let (status, body) = get(state, "/api/activity").await;
        assert_eq!(status, StatusCode::OK);
        let v = parse(&body);
        assert_eq!(v["events"].as_array().unwrap().len(), 10);
    }

    #[tokio::test]
    async fn limit_zero_returns_empty() {
        let state = minimal_state().await;
        let buf: &ActivityRingBuffer = state.activity_feed.as_ref();
        for i in 1..=5_i64 {
            buf.push(ev(i));
        }
        let (status, body) = get(state, "/api/activity?limit=0").await;
        assert_eq!(status, StatusCode::OK);
        let v = parse(&body);
        assert!(v["events"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn limit_non_numeric_returns_400() {
        let state = minimal_state().await;
        let (status, _) = get(state, "/api/activity?limit=foo").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn oversized_limit_is_clamped() {
        let state = minimal_state().await;
        let buf: &ActivityRingBuffer = state.activity_feed.as_ref();
        for i in 1..=10_i64 {
            buf.push(ev(i));
        }
        let (status, body) = get(state, "/api/activity?limit=999999").await;
        assert_eq!(status, StatusCode::OK);
        let v = parse(&body);
        // Only 10 events were pushed; the server clamps at CAPACITY=50.
        assert_eq!(v["events"].as_array().unwrap().len(), 10);
    }

    #[tokio::test]
    async fn restart_with_no_pushes_returns_empty() {
        // Pins the "no persistence across restarts" contract documented in
        // What We're NOT Doing. If a future change adds persistence, this
        // test must change deliberately.
        let state = minimal_state().await;
        let (status, body) = get(state, "/api/activity?limit=5").await;
        assert_eq!(status, StatusCode::OK);
        let v = parse(&body);
        assert!(v["events"].as_array().unwrap().is_empty());
    }
}
