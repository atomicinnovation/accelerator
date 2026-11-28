use std::path::{Path, PathBuf};
use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::compose::{load, Params};
use accelerator_visualiser::config::Config;
use accelerator_visualiser::server::AppState;

mod common;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn project_with_config(dir: &Path, config: &str) {
    let accelerator = dir.join(".accelerator");
    std::fs::create_dir_all(accelerator.join("tmp")).unwrap();
    std::fs::write(accelerator.join("config.md"), config).unwrap();
}

fn compose(dir: &Path) -> Result<Config, String> {
    load(Params {
        cwd: dir.to_path_buf(),
        plugin_root: repo_root(),
        owner_pid: 0,
        owner_start_time: None,
        host: "127.0.0.1".to_string(),
    })
    .map_err(|error| error.to_string())
}

async fn serve(cfg: Config) -> Result<(), String> {
    AppState::build(cfg, Arc::new(Activity::new()))
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

async fn compose_and_serve(dir: &Path) -> Result<(), String> {
    serve(compose(dir)?).await
}

fn assert_names(refusal: Result<(), String>, container: &str, contained: &str) {
    let message = refusal.expect_err("overlapping doc roots were served");
    assert!(
        message.contains(&format!("paths.{container} ("))
            && message.contains(&format!("paths.{contained} ("))
            && message.contains("may not equal or contain one another"),
        "the refusal does not name both keys: {message}"
    );
}

#[tokio::test]
async fn an_overlapping_configuration_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    project_with_config(tmp.path(), "---\npaths:\n  notes: meta/plans\n---\n");

    assert_names(compose_and_serve(tmp.path()).await, "plans", "notes");
}

#[cfg(unix)]
#[tokio::test]
async fn a_root_aliased_by_a_symlink_into_another_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    project_with_config(tmp.path(), "---\n---\n");
    let topics = tmp.path().join("meta/research/topics");
    std::fs::create_dir_all(&topics).unwrap();
    std::os::unix::fs::symlink(&topics, tmp.path().join("meta/notes")).unwrap();

    assert_names(
        compose_and_serve(tmp.path()).await,
        "research_topics",
        "notes",
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_missing_root_under_a_symlink_into_another_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    project_with_config(
        tmp.path(),
        "---\npaths:\n  notes: meta/alias/notes\n---\n",
    );
    let topics = tmp.path().join("meta/research/topics");
    std::fs::create_dir_all(&topics).unwrap();
    std::os::unix::fs::symlink(&topics, tmp.path().join("meta/alias")).unwrap();

    assert_names(
        compose_and_serve(tmp.path()).await,
        "research_topics",
        "notes",
    );
}

#[tokio::test]
async fn a_root_supplied_directly_on_the_config_is_checked_too() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = common::seeded_cfg(tmp.path());
    cfg.doc_paths
        .insert("notes".into(), tmp.path().join("meta/plans/notes"));

    assert_names(serve(cfg).await, "plans", "notes");
}
