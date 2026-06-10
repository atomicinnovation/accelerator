use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::server::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditorConfigBody {
    /// `None` → no editor configured → frontend renders the disabled state.
    editor: Option<String>,
    /// Resolved JetBrains project name: configured `editor_project`, else the
    /// basename of `project_root`. Always present so the frontend never derives it.
    editor_project: String,
}

pub(crate) async fn get_editor_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let editor_project = state
        .cfg
        .editor_project
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            state
                .cfg
                .project_root
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
    Json(EditorConfigBody {
        editor: state.cfg.editor.clone().filter(|s| !s.trim().is_empty()),
        editor_project,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::Path;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::activity::Activity;
    use crate::config::Config;
    use crate::server::build_router;

    fn minimal_config(project_root: &Path, tmp: &Path) -> Config {
        Config {
            plugin_root: tmp.to_path_buf(),
            plugin_version: "test".into(),
            project_root: project_root.to_path_buf(),
            tmp_path: tmp.to_path_buf(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: tmp.join("server.log"),
            doc_paths: HashMap::new(),
            templates: HashMap::new(),
            work_item: None,
            kanban_columns: None,
            idle_timeout: None,
            editor: None,
            editor_project: None,
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
                    .uri("/api/editor/config")
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
    async fn absent_editor_is_null_and_project_defaults_to_basename() {
        let tmp = tempfile::tempdir().unwrap();
        let project_root = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_root).unwrap();
        let state = build_state(minimal_config(&project_root, tmp.path())).await;
        let body = get_json(state).await;
        assert!(body["editor"].is_null());
        assert_eq!(body["editorProject"], "my-project");
    }

    #[tokio::test]
    async fn configured_editor_and_project_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let project_root = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_root).unwrap();
        let mut cfg = minimal_config(&project_root, tmp.path());
        cfg.editor = Some("cursor".into());
        cfg.editor_project = Some("myrepo".into());
        let state = build_state(cfg).await;
        let body = get_json(state).await;
        assert_eq!(body["editor"], "cursor");
        assert_eq!(body["editorProject"], "myrepo");
    }

    #[tokio::test]
    async fn whitespace_only_editor_treated_as_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let project_root = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_root).unwrap();
        let mut cfg = minimal_config(&project_root, tmp.path());
        cfg.editor = Some("   ".into());
        let state = build_state(cfg).await;
        let body = get_json(state).await;
        assert!(body["editor"].is_null());
    }

    #[tokio::test]
    async fn whitespace_only_project_falls_back_to_basename() {
        let tmp = tempfile::tempdir().unwrap();
        let project_root = tmp.path().join("my-project");
        std::fs::create_dir_all(&project_root).unwrap();
        let mut cfg = minimal_config(&project_root, tmp.path());
        cfg.editor_project = Some("  ".into());
        let state = build_state(cfg).await;
        let body = get_json(state).await;
        assert_eq!(body["editorProject"], "my-project");
    }
}
