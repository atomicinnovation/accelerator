use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn exits_2_when_config_missing() {
    let mut cmd = Command::cargo_bin("accelerator-visualiser").unwrap();
    cmd.args(["--config", "/nonexistent/config.json"]);
    cmd.assert().code(2);
}

#[test]
fn exits_1_when_idle_timeout_invalid() {
    // An otherwise-valid config whose only fault is `idle_timeout: "soon"`
    // must fail fast at `resolve_idle_limit_ms` (ConfigError → AppStateError →
    // ServerError::Startup → ExitCode::from(1)). Assert code == 1 SPECIFICALLY,
    // not merely non-zero: a missing/unreadable config exits 2, so a loose
    // non-zero assertion would green-light a regression where the resolver
    // never runs.
    //
    // Paths point inside a tempdir so log::init (which create_dir_all's the log
    // parent before server::run) succeeds; the run then fails only at the
    // resolver. The doc_paths/templates are never touched — the resolver
    // rejects the timeout before the indexer builds.
    let dir = tempfile::tempdir().unwrap();
    let tmp_path = dir.path().join("visualiser");
    let log_path = tmp_path.join("server.log");
    let config = serde_json::json!({
        "plugin_root": dir.path(),
        "plugin_version": "0.0.0-test",
        "project_root": dir.path(),
        "tmp_path": tmp_path,
        "host": "127.0.0.1",
        "owner_pid": 0,
        "owner_start_time": null,
        "log_path": log_path,
        "doc_paths": {},
        "templates": {},
        "idle_timeout": "soon"
    });
    let config_path = dir.path().join("config.json");
    std::fs::write(&config_path, serde_json::to_vec_pretty(&config).unwrap())
        .unwrap();

    let mut cmd = Command::cargo_bin("accelerator-visualiser").unwrap();
    cmd.args(["--config", config_path.to_str().unwrap()]);
    cmd.assert().code(1);
}
