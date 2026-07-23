use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::config::{
    Config, RawWorkItemConfig, TemplateTiers,
};
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::collections::HashMap;
use tower::ServiceExt;

fn build_project_pattern_config(tmp: &std::path::Path) -> Config {
    let work = tmp.join("meta/work");
    std::fs::create_dir_all(&work).unwrap();

    std::fs::write(
        work.join("PROJ-0042-foo.md"),
        "---\nid: \"PROJ-0042\"\ntitle: Foo Work Item\nstatus: ready\n---\n# body\n",
    )
    .unwrap();
    std::fs::write(
        work.join("PROJ-0007-bar.md"),
        "---\nid: \"PROJ-0007\"\ntitle: Bar Work Item\nstatus: done\n---\n# body\n",
    )
    .unwrap();

    // Mixed-pattern: a bare-numeric file that predates the project prefix and
    // carries NO `id:`. Post-contract, the filename is no longer an identity
    // fallback, so this file resolves to no work-item id (see the dedicated
    // test below).
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
                config_override_source: None,
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
        idle_timeout: None,
        editor: None,
        editor_project: None,
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
    assert_eq!(
        foo["slug"], "foo",
        "slug for PROJ-0042-foo.md should be 'foo'"
    );

    let bar = by_id.get("PROJ-0007").expect("PROJ-0007 must be indexed");
    assert_eq!(
        bar["slug"], "bar",
        "slug for PROJ-0007-bar.md should be 'bar'"
    );
}

#[tokio::test]
async fn bare_numeric_file_without_id_has_no_resolved_work_item_id() {
    // Contract-phase behaviour: the filename is no longer an identity fallback.
    // A bare-numeric file in a project-prefixed workspace that carries no `id:`
    // is still indexed as a work-item doc (by slug) but resolves to NO
    // work-item id — previously it was admitted as PROJ-0001 via the removed
    // two-pass filename fallback. Migrating the file (adding `id:`) restores it.
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

    // The filename is no longer a fallback identity source: nothing resolves to
    // the would-be PROJ-0001.
    assert!(
        docs.iter()
            .all(|d| d["workItemId"].as_str() != Some("PROJ-0001")),
        "bare-numeric file must not be filename-resolved to PROJ-0001"
    );
    // It is still indexed as a work-item doc (slug derived from the filename),
    // but with no resolved work-item id.
    let legacy = docs
        .iter()
        .find(|d| d["slug"] == "legacy")
        .expect("0001-legacy.md is still indexed as a work-item doc");
    assert!(
        legacy["workItemId"].is_null(),
        "bare-numeric file without id: must have a null work-item id, got {:?}",
        legacy["workItemId"]
    );
}

#[tokio::test]
async fn default_numeric_pattern_indexes_bare_numeric_files() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp = tmp.path();
    let work = tmp.join("meta/work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(
        work.join("0042-ship-it.md"),
        "---\nid: \"0042\"\ntitle: Ship It\nstatus: ready\n---\n",
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
                config_override_source: None,
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
        idle_timeout: None,
        editor: None,
        editor_project: None,
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
