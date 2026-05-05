use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::server::AppState;

#[derive(Debug, Serialize)]
pub(crate) struct KanbanConfigBody {
    columns: Vec<KanbanColumnDto>,
}

#[derive(Debug, Serialize)]
pub(crate) struct KanbanColumnDto {
    key: String,
    label: String,
}

pub(crate) async fn get_kanban_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let columns = state
        .kanban_columns
        .iter()
        .map(|c| KanbanColumnDto { key: c.key.clone(), label: c.label.clone() })
        .collect();
    Json(KanbanConfigBody { columns })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::activity::Activity;
    use crate::config::Config;
    use crate::server::build_router;

    fn minimal_config(tmp: &std::path::Path) -> Config {
        Config {
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
            work_item: None,
            kanban_columns: None,
        }
    }

    async fn build_state(cfg: Config) -> Arc<AppState> {
        let activity = Arc::new(Activity::new());
        AppState::build(cfg, activity).await.unwrap()
    }

    async fn get_json(state: Arc<AppState>) -> serde_json::Value {
        let app = build_router(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/kanban/config")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn returns_seven_defaults_when_no_kanban_columns_configured() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(minimal_config(tmp.path())).await;
        let body = get_json(state).await;
        let cols = body["columns"].as_array().unwrap();
        assert_eq!(cols.len(), 7);
        assert_eq!(cols[0]["key"], "draft");
        assert_eq!(cols[0]["label"], "Draft");
        assert_eq!(cols[2]["key"], "in-progress");
        assert_eq!(cols[2]["label"], "In progress");
        assert_eq!(cols[6]["key"], "abandoned");
    }

    #[tokio::test]
    async fn returns_configured_columns_with_derived_labels() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = minimal_config(tmp.path());
        cfg.kanban_columns = Some(vec!["ready".into(), "in-progress".into(), "done".into()]);
        let state = build_state(cfg).await;
        let body = get_json(state).await;
        let cols = body["columns"].as_array().unwrap();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0]["key"], "ready");
        assert_eq!(cols[0]["label"], "Ready");
        assert_eq!(cols[1]["key"], "in-progress");
        assert_eq!(cols[1]["label"], "In progress");
    }

    #[tokio::test]
    async fn empty_kanban_columns_rejects_at_boot() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = minimal_config(tmp.path());
        cfg.kanban_columns = Some(vec![]);
        let activity = Arc::new(Activity::new());
        let result = AppState::build(cfg, activity).await;
        assert!(result.is_err(), "build must fail with empty kanban_columns");
        let err = result.err().unwrap();
        assert!(
            matches!(err, crate::server::AppStateError::Config(
                crate::config::ConfigError::EmptyKanbanColumns
            )),
            "expected EmptyKanbanColumns"
        );
    }
}
