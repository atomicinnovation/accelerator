use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::server::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkItemConfigBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

pub(crate) async fn get_work_item_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let key = state
        .cfg
        .work_item
        .configured()
        .and_then(|scheme| scheme.key())
        .map(|key| key.as_str().to_owned());
    let problem = state.work_item_problem();
    Json(WorkItemConfigBody { key, problem })
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
            work_item: crate::config::WorkItemScheme::Default,
            kanban_columns: None,
            idle_timeout: None,
            editor: None,
            editor_project: None,
        }
    }

    #[tokio::test]
    async fn returns_empty_object_when_no_project_code_configured() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = minimal_config(tmp.path());
        let activity = Arc::new(Activity::new());
        let state =
            crate::server::AppState::build(cfg, activity).await.unwrap();
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
        assert!(
            v["key"].is_null() || !v.as_object().unwrap().contains_key("key"),
            "key should be absent or null when unconfigured"
        );
    }

    #[tokio::test]
    async fn returns_the_key_when_configured() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = minimal_config(tmp.path());
        cfg.work_item = crate::config::WorkItemScheme::Configured(
            crate::config::scheme_for_test(
                "{project}-{number:04d}",
                Some("PROJ"),
            ),
        );
        let activity = Arc::new(Activity::new());
        let state =
            crate::server::AppState::build(cfg, activity).await.unwrap();
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
        assert_eq!(v["key"], "PROJ");
    }

    async fn config_body(cfg: Config) -> serde_json::Value {
        let activity = Arc::new(Activity::new());
        let state =
            crate::server::AppState::build(cfg, activity).await.unwrap();
        let resp = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/work-item/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn an_unreadable_scheme_is_reported_as_the_problem() {
        let tmp = tempfile::tempdir().unwrap();
        let mut cfg = minimal_config(tmp.path());
        cfg.work_item = crate::config::WorkItemScheme::Unreadable(
            "E_WORK_ID_PATTERN_INVALID: work.id_pattern 'x': bad".into(),
        );

        let body = config_body(cfg).await;

        assert_eq!(
            body["problem"],
            "E_WORK_ID_PATTERN_INVALID: work.id_pattern 'x': bad"
        );
    }

    #[tokio::test]
    async fn a_corpus_off_the_pattern_is_reported_as_the_problem() {
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("meta/work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(
            work.join("PP-0001-a.md"),
            "---\nid: \"PP-0001\"\n---\n",
        )
        .unwrap();
        let mut cfg = minimal_config(tmp.path());
        cfg.doc_paths.insert("work".into(), work);
        cfg.work_item = crate::config::WorkItemScheme::Configured(
            crate::config::scheme_for_test("{key}_{number:04d}", Some("PP")),
        );

        let body = config_body(cfg).await;

        let problem = body["problem"].as_str().unwrap_or_default();
        assert!(
            problem.starts_with("E_REKEY_REQUIRED: 1 work item"),
            "{body}"
        );
        assert!(problem.contains("meta/work/PP-0001-a.md"), "{body}");
    }

    #[tokio::test]
    async fn a_malformed_work_item_is_reported_as_the_problem() {
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("meta/work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("0001.md"), "---\nid: \"0001\"\n---\n")
            .unwrap();
        let mut cfg = minimal_config(tmp.path());
        cfg.doc_paths.insert("work".into(), work);

        let body = config_body(cfg).await;

        let problem = body["problem"].as_str().unwrap_or_default();
        assert!(
            problem.starts_with(
                "E_WORK_ITEM_MALFORMED: meta/work/0001.md has `id` '0001'"
            ),
            "{body}"
        );
    }

    #[tokio::test]
    async fn the_problem_follows_the_corpus_after_boot() {
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("meta/work");
        std::fs::create_dir_all(&work).unwrap();
        let mut cfg = minimal_config(tmp.path());
        cfg.doc_paths.insert("work".into(), work.clone());
        let state =
            crate::server::AppState::build(cfg, Arc::new(Activity::new()))
                .await
                .unwrap();
        let app = build_router(state);
        let problem = |app: axum::Router| async move {
            let resp = app
                .oneshot(
                    Request::builder()
                        .uri("/api/work-item/config")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let bytes = resp.into_body().collect().await.unwrap().to_bytes();
            let body: serde_json::Value =
                serde_json::from_slice(&bytes).unwrap();
            body["problem"].as_str().map(str::to_owned)
        };

        std::fs::write(work.join("42-a.md"), "---\nid: \"42\"\n---\n").unwrap();
        let off_pattern = problem(app.clone()).await;
        std::fs::remove_file(work.join("42-a.md")).unwrap();
        std::fs::write(work.join("0042-a.md"), "---\nid: \"0042\"\n---\n")
            .unwrap();
        let converged = problem(app).await;

        assert!(
            off_pattern
                .as_deref()
                .is_some_and(|p| p.starts_with("E_REKEY_REQUIRED")),
            "{off_pattern:?}"
        );
        assert_eq!(converged, None);
    }

    #[tokio::test]
    async fn a_converged_corpus_reports_no_problem() {
        let tmp = tempfile::tempdir().unwrap();
        let body = config_body(minimal_config(tmp.path())).await;
        assert!(body.get("problem").is_none(), "{body}");
    }
}
