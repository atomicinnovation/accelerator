use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::server::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkItemConfigBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_project_code: Option<String>,
}

pub(crate) async fn get_work_item_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let default_project_code = state
        .cfg
        .work_item
        .as_ref()
        .and_then(|w| w.default_project_code.clone());
    Json(WorkItemConfigBody { default_project_code })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt as _;

    use crate::{activity::Activity, config::Config, server::build_router};

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

    #[tokio::test]
    async fn returns_empty_object_when_no_project_code_configured() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = minimal_config(tmp.path());
        let activity = Arc::new(Activity::new());
        let state = crate::server::AppState::build(cfg, activity).await.unwrap();
        let app = build_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/work-item/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(v["defaultProjectCode"].is_null() || !v.as_object().unwrap().contains_key("defaultProjectCode"),
            "defaultProjectCode should be absent or null when unconfigured");
    }

    #[tokio::test]
    async fn returns_default_project_code_when_configured() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = minimal_config(tmp.path());
        cfg.work_item = Some(crate::config::RawWorkItemConfig {
            scan_regex: "^PROJ-([0-9]+)-".into(),
            id_pattern: "{project}-{number:04d}".into(),
            default_project_code: Some("PROJ".into()),
        });
        let activity = Arc::new(Activity::new());
        let state = crate::server::AppState::build(cfg, activity).await.unwrap();
        let app = build_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/work-item/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["defaultProjectCode"], "PROJ");
    }
}
