#[cfg(feature = "dev-frontend")]
mod tests {
    use accelerator_visualiser::server::AppState;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt as _;
    use std::collections::HashMap;
    use tower::ServiceExt as _;

    async fn minimal_state(tmp: &std::path::Path) -> std::sync::Arc<AppState> {
        let cfg = accelerator_visualiser::config::Config {
            plugin_root: tmp.to_path_buf(),
            plugin_version: "test".into(),
            project_root: tmp.to_path_buf(),
            tmp_path: tmp.to_path_buf(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: tmp.join("server.log"),
            doc_paths: HashMap::new(),
            templates: HashMap::new(),
        };
        let activity = std::sync::Arc::new(accelerator_visualiser::activity::Activity::new());
        AppState::build(cfg, activity).await.unwrap()
    }

    #[tokio::test]
    async fn spa_route_returns_html() {
        let state_tmp = tempfile::tempdir().unwrap();
        let state = minimal_state(state_tmp.path()).await;

        let dist = tempfile::tempdir().unwrap();
        std::fs::write(
            dist.path().join("index.html"),
            "<!doctype html><html>app</html>",
        )
        .unwrap();

        let app = accelerator_visualiser::server::build_router_with_dist(
            state,
            dist.path().to_path_buf(),
        );

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/library")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(
            text.contains("<!doctype html") || text.contains("<!DOCTYPE html"),
            "expected HTML, got: {text:.200}"
        );
    }
}
