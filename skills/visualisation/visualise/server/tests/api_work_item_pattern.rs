use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::config::{Config, RawWorkItemConfig, TemplateTiers};
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use std::collections::HashMap;

fn build_project_pattern_config(tmp: &std::path::Path) -> Config {
    let work = tmp.join("meta/work");
    std::fs::create_dir_all(&work).unwrap();

    std::fs::write(
        work.join("PROJ-0042-foo.md"),
        "---\ntitle: Foo Work Item\nstatus: ready\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        work.join("PROJ-0007-bar.md"),
        "---\ntitle: Bar Work Item\nstatus: done\n---\n# body\n",
    )
    .unwrap();

    // Mixed-pattern: a legacy bare-numeric file that predates the project prefix.
    std::fs::write(
        work.join("0001-legacy.md"),
        "---\ntitle: Legacy Work Item\nstatus: todo\n---\n# body\n",
    )
    .unwrap();

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
            },
        );
    }

    let tmp_dir = tmp.join("meta/tmp/visualiser");
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let mut doc_paths = HashMap::new();
    doc_paths.insert("work".into(), work);

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
        work_item: Some(RawWorkItemConfig {
            // Captures only the digit run; project code literal is outside group 1.
            scan_regex: "^PROJ-([0-9]+)-".to_string(),
            id_pattern: "{project}-{number:04d}".to_string(),
            default_project_code: Some("PROJ".to_string()),
        }),
        kanban_columns: None,
    }
}

async fn json_body(res: axum::response::Response) -> serde_json::Value {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
async fn project_pattern_work_items_have_correct_slug_and_id() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = build_project_pattern_config(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=work-items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let docs = body["docs"].as_array().expect("docs array");

    // Both project-prefixed files should appear.
    let by_id: HashMap<String, &serde_json::Value> = docs
        .iter()
        .filter_map(|d| {
            let id = d["workItemId"].as_str()?;
            Some((id.to_string(), d))
        })
        .collect();

    let foo = by_id.get("PROJ-0042").expect("PROJ-0042 must be indexed");
    assert_eq!(foo["slug"], "foo", "slug for PROJ-0042-foo.md should be 'foo'");

    let bar = by_id.get("PROJ-0007").expect("PROJ-0007 must be indexed");
    assert_eq!(bar["slug"], "bar", "slug for PROJ-0007-bar.md should be 'bar'");
}

#[tokio::test]
async fn legacy_bare_numeric_files_admitted_via_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = build_project_pattern_config(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=work-items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let docs = body["docs"].as_array().expect("docs array");

    // The legacy bare-numeric file (0001-legacy.md) should be admitted under
    // the canonical project-prefixed ID via the two-pass fallback rule.
    let legacy = docs
        .iter()
        .find(|d| d["workItemId"].as_str() == Some("PROJ-0001"))
        .expect("legacy bare-numeric file must be admitted as PROJ-0001");
    assert_eq!(legacy["slug"], "legacy");
}

#[tokio::test]
async fn default_numeric_pattern_indexes_bare_numeric_files() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp = tmp.path();
    let work = tmp.join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(
        work.join("0042-ship-it.md"),
        "---\ntitle: Ship It\nstatus: ready\n---\n",
    )
    .unwrap();

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
            },
        );
    }
    let tmp_dir = tmp.join("meta/tmp/visualiser");
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let mut doc_paths = HashMap::new();
    doc_paths.insert("work".into(), work);

    let cfg = Config {
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
    };

    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/docs?type=work-items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = json_body(res).await;
    let docs = body["docs"].as_array().expect("docs array");

    let entry = docs
        .iter()
        .find(|d| d["workItemId"].as_str() == Some("0042"))
        .expect("0042-ship-it.md must be indexed with id '0042'");
    assert_eq!(entry["slug"], "ship-it");
}
