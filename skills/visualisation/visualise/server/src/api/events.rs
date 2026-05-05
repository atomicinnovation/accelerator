//! `GET /api/events` — SSE stream of doc-changed / doc-invalid events.

use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;

use crate::server::AppState;

pub async fn events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.sse_hub.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(payload) => {
            let data = serde_json::to_string(&payload).ok()?;
            Some(Ok::<Event, Infallible>(Event::default().data(data)))
        }
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
            tracing::warn!(dropped = n, "SSE subscriber lagged; messages dropped");
            None
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use crate::docs::DocTypeKey;
    use crate::server::AppState;
    use crate::sse_hub::SsePayload;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
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
        };
        let activity = Arc::new(crate::activity::Activity::new());
        AppState::build(cfg, activity).await.unwrap()
    }

    #[tokio::test]
    async fn events_endpoint_returns_text_event_stream() {
        let state = minimal_state().await;
        let app = crate::server::build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/events")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let ct = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.starts_with("text/event-stream"),
            "expected text/event-stream, got {ct}",
        );
    }

    #[tokio::test]
    async fn hub_event_arrives_on_sse_stream() {
        use http_body_util::BodyExt as _;

        let state = minimal_state().await;
        let hub = state.sse_hub.clone();
        let app = crate::server::build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/events")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        hub.broadcast(SsePayload::DocChanged {
            doc_type: DocTypeKey::Plans,
            path: "meta/plans/foo.md".into(),
            etag: Some("sha256-abc".into()),
        });

        let frame = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            response.into_body().frame(),
        )
        .await
        .expect("timed out waiting for first SSE frame")
        .expect("body error")
        .expect("stream ended before first frame");

        let bytes = frame.into_data().expect("expected data frame");
        let text = std::str::from_utf8(&bytes).unwrap();

        assert!(
            text.contains("doc-changed"),
            "SSE frame did not contain 'doc-changed': {text}",
        );
        assert!(text.contains("sha256-abc"), "frame: {text}");
    }
}
