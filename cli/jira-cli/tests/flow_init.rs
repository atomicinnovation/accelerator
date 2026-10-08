//! The `init verify` flow: it verifies credentials against `/myself`, caches the
//! site identity, stamps the `verified` outcome, and — the load-bearing
//! guarantee — never prints the token on any exit path. The
//! `Secret` redaction in `config::credentials` is why the guarantee holds.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};

use http_test_support::{MockHTTPServer, RequestKey, Route};
use serde_json::{json, Value};
use support::{Token, TOKEN_SENTINEL};

const MYSELF: &str = "/rest/api/3/myself";
const PROJECTS: &str = "/rest/api/3/project";
const FIELDS: &str = "/rest/api/3/field";

fn no_leak(output: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains(TOKEN_SENTINEL), "token leaked to stdout");
    assert!(!stderr.contains(TOKEN_SENTINEL), "token leaked to stderr");
}

#[test]
fn verify_caches_the_site_and_stamps_the_outcome() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(MYSELF),
        Route::Json {
            status: 200,
            body: r#"{"accountId":"acc-1","displayName":"Ada"}"#.to_owned(),
        },
    );
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "verify"]);
    assert!(
        output.status.success(),
        "verify exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    no_leak(&output);

    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        envelope.pointer("/outcome").and_then(Value::as_str),
        Some("verified")
    );
    assert_eq!(
        envelope.pointer("/accountId").and_then(Value::as_str),
        Some("acc-1")
    );

    let cache = dir
        .path()
        .join(".accelerator/state/integrations/jira/site.json");
    assert!(cache.exists(), "the site cache is written");
}

#[test]
fn a_verify_failure_never_leaks_the_token() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get(MYSELF), Route::Status(401));
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "verify"]);
    assert_eq!(output.status.code(), Some(61), "verify-failed maps to 61");
    no_leak(&output);
}

#[test]
fn a_missing_token_never_leaks_and_maps_to_no_token() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get(MYSELF), Route::Status(200));
    let dir = support::scratch(support::CONFIG);

    let output = support::run_with(
        dir.path(),
        &["init", "verify"],
        Some(&server.base_url()),
        &Token::Absent,
    );
    assert_eq!(output.status.code(), Some(24), "no token → NO_TOKEN");
    no_leak(&output);
}

fn discovering_server() -> MockHTTPServer {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(PROJECTS),
        Route::Json {
            status: 200,
            body: r#"[{"key":"ENG","id":"10000","name":"Eng"}]"#.to_owned(),
        },
    );
    server.route(
        RequestKey::get(FIELDS),
        Route::Json {
            status: 200,
            body: r#"[{"id":"customfield_10016","name":"Story Points",
                       "custom":true,"schema":{"type":"number"}}]"#
                .to_owned(),
        },
    );
    server
}

fn state_dir(dir: &Path) -> PathBuf {
    dir.join(".accelerator/state/integrations/jira")
}

fn cached(dir: &Path, name: &str) -> Value {
    let raw = std::fs::read_to_string(state_dir(dir).join(name))
        .expect("the cache file is written");
    serde_json::from_str(&raw).expect("the cache file is JSON")
}

fn discovered_projects() -> Value {
    json!({
        "projects": [{"id": "10000", "key": "ENG", "name": "Eng"}],
        "site": "127.0.0.1"
    })
}

fn discovered_fields() -> Value {
    json!({
        "fields": [{
            "id": "customfield_10016",
            "key": null,
            "name": "Story Points",
            "schema": {"type": "number"},
            "slug": "story-points"
        }],
        "site": "127.0.0.1"
    })
}

#[test]
fn discover_reports_the_discovered_projects_and_fields() {
    let server = discovering_server();
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "discover"]);

    assert!(
        output.status.success(),
        "discover exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("json stdout");
    assert_eq!(
        envelope,
        json!({
            "outcome": "discovered",
            "projects": discovered_projects(),
            "fields": discovered_fields()
        })
    );
}

#[test]
fn discover_caches_the_projects_and_fields_under_a_version_marker() {
    let server = discovering_server();
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "discover"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(cached(dir.path(), "projects.json"), discovered_projects());
    assert_eq!(cached(dir.path(), "fields.json"), discovered_fields());
    assert_eq!(
        cached(dir.path(), ".cache-version.json"),
        json!({"version": 1})
    );
}

#[test]
fn discover_leaves_the_state_dir_without_a_scaffold() {
    let server = discovering_server();
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "discover"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut entries: Vec<String> = std::fs::read_dir(state_dir(dir.path()))
        .expect("the state dir is readable")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        [".cache-version.json", "fields.json", "projects.json"]
    );
}

#[cfg(unix)]
#[test]
fn discover_under_a_read_only_state_dir_fails_with_a_cache_io_error() {
    use std::os::unix::fs::PermissionsExt as _;

    let server = discovering_server();
    let dir = support::scratch(support::CONFIG);
    let state = state_dir(dir.path());
    std::fs::create_dir_all(&state).expect("mkdir state dir");
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o555))
        .expect("seal the state dir");

    let output = support::run(dir.path(), &server, &["init", "discover"]);

    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o755))
        .expect("unseal the state dir");
    assert_eq!(output.status.code(), Some(1));
    let lock = std::fs::canonicalize(&state)
        .expect("the state dir resolves")
        .join(".lock");
    let lock = lock.display();
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "E_CACHE_IO: {lock}: cannot write under '{lock}': not writable\n"
        )
    );
}
