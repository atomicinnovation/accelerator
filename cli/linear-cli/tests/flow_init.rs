//! `init verify` validates credentials without ever printing the token
//! and persists `viewer.json`; `init discover` persists complete entries
//! for the base team and every synced team into `catalogue.json`. The
//! `Secret`-redaction invariant is why the no-token guarantee holds; the
//! binary owns cache production so the repointed skill needs no `Write` grant.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use cli_test_support::Scenario;
use http_test_support::MockServer;
use serde_json::Value;

fn install(server: &MockServer, name: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios")
        .join(format!("{name}.json"));
    Scenario::load(&path).expect("scenario").install(server);
}

fn state_file(dir: &Path, name: &str) -> std::path::PathBuf {
    dir.join(".accelerator/state/integrations/linear")
        .join(name)
}

fn seed_catalogue(dir: &Path, content: &str) {
    let path = state_file(dir, "catalogue.json");
    std::fs::create_dir_all(path.parent().expect("state dir"))
        .expect("mkdir state dir");
    std::fs::write(path, content).expect("seed the catalogue");
}

fn read_catalogue(dir: &Path) -> Value {
    let raw = std::fs::read_to_string(state_file(dir, "catalogue.json"))
        .expect("catalogue.json written");
    serde_json::from_str(&raw).expect("catalogue is JSON")
}

fn discover(dir: &Path, server: &MockServer) -> std::process::Output {
    support::run(
        dir,
        server,
        &["init", "discover", "--team-id", "team-x-uuid"],
    )
}

fn team<'a>(catalogue: &'a Value, id: &str) -> Option<&'a Value> {
    catalogue["teams"]
        .as_array()
        .expect("teams")
        .iter()
        .find(|entry| entry["id"] == id)
}

const SYNCED_TEAM_Y: &str = r#"{
  "team": {"id": "team-x-uuid", "key": "BLA", "name": "Team X"},
  "workflowStates": [],
  "teams": [{"key": "OPS", "id": "team-y-uuid", "name": "Team Y"}]
}"#;

#[test]
fn init_verify_persists_the_viewer_without_leaking_the_token() {
    let server = MockServer::start();
    install(&server, "viewer-200");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "verify"]);

    assert!(
        output.status.success(),
        "init verify exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains(support::TOKEN_SENTINEL),
        "the token must never reach stdout: {stdout}"
    );
    assert!(
        !stderr.contains(support::TOKEN_SENTINEL),
        "the token must never reach stderr: {stderr}"
    );
    let doc: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(doc.get("outcome").and_then(Value::as_str), Some("verified"));
    assert_eq!(doc.get("name").and_then(Value::as_str), Some("Test User"));

    // The binary owns cache production: viewer.json is written and the token is
    // not in it either.
    let viewer = std::fs::read_to_string(state_file(dir.path(), "viewer.json"))
        .expect("viewer.json written");
    assert!(viewer.contains("Test User"));
    assert!(!viewer.contains(support::TOKEN_SENTINEL));
}

#[test]
fn init_discover_persists_the_catalogue() {
    let server = MockServer::start();
    install(&server, "team-states-200");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["init", "discover", "--team-id", "team-x-uuid"],
    );

    assert!(
        output.status.success(),
        "init discover exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let doc: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        doc.get("outcome").and_then(Value::as_str),
        Some("discovered")
    );

    let catalogue =
        std::fs::read_to_string(state_file(dir.path(), "catalogue.json"))
            .expect("catalogue.json written");
    let parsed: Value =
        serde_json::from_str(&catalogue).expect("catalogue is JSON");
    assert_eq!(
        parsed.pointer("/team/key").and_then(Value::as_str),
        Some("BLA")
    );
    let projected: Vec<&str> = parsed["workflowStates"]
        .as_array()
        .expect("the legacy states projection")
        .iter()
        .filter_map(|state| state["name"].as_str())
        .collect();
    assert_eq!(projected, vec!["In Progress", "Todo"]);
    // The catalogue is committed, so it is not gitignored; viewer.json is.
    let gitignore =
        std::fs::read_to_string(state_file(dir.path(), ".gitignore"))
            .expect(".gitignore scaffolded");
    assert!(gitignore.lines().any(|line| line == "viewer.json"));
    assert!(!gitignore.lines().any(|line| line == "catalogue.json"));
}

#[test]
fn init_list_teams_renders_the_teams_with_the_listed_keyword() {
    let server = MockServer::start();
    install(&server, "teams-200");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "list-teams"]);

    assert!(
        output.status.success(),
        "init list-teams exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        stdout.get("outcome").and_then(Value::as_str),
        Some("listed")
    );
    assert!(
        stdout.pointer("/teams").is_some(),
        "the team list is rendered: {stdout}"
    );
}

#[test]
fn init_discover_writes_complete_entries_for_base_and_synced_teams() {
    let server = MockServer::start();
    install(&server, "team-states-200");
    let dir = support::scratch(support::CONFIG);
    seed_catalogue(dir.path(), SYNCED_TEAM_Y);

    let output = discover(dir.path(), &server);

    assert!(
        output.status.success(),
        "init discover exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let catalogue = read_catalogue(dir.path());
    assert_eq!(catalogue["baseTeam"], "team-x-uuid");
    for id in ["team-x-uuid", "team-y-uuid"] {
        let entry = team(&catalogue, id).expect("a synced team entry");
        for section in ["states", "labels", "members", "projects"] {
            assert!(entry[section].is_array(), "{id} lacks {section}: {entry}");
        }
    }
    assert_eq!(catalogue["labels"][0]["name"], "Security");
    let sent = server
        .bodies(&http_test_support::RequestKey::graphql("TeamIdentities"));
    let request: Value = serde_json::from_slice(&sent[0]).expect("JSON");
    assert_eq!(
        request["variables"]["ids"],
        serde_json::json!(["team-x-uuid", "team-y-uuid"])
    );

    let stdout: Value = serde_json::from_slice(&output.stdout).expect("JSON");
    assert_eq!(stdout["outcome"], "discovered");
    assert_eq!(stdout["team"]["key"], "BLA");
    assert_eq!(
        stdout["teams"],
        serde_json::json!([
            { "id": "team-x-uuid", "key": "BLA", "name": "Team X",
              "states": 2, "labels": 1, "members": 1, "projects": 1 },
            { "id": "team-y-uuid", "key": "OPS", "name": "Team Y",
              "states": 1, "labels": 0, "members": 1, "projects": 0 },
        ])
    );
    assert_eq!(stdout["workspaceLabels"], 1);
}

#[test]
fn init_discover_never_catalogues_other_visible_teams() {
    let server = MockServer::start();
    install(&server, "team-states-200");
    let dir = support::scratch(support::CONFIG);
    seed_catalogue(dir.path(), SYNCED_TEAM_Y);

    let output = discover(dir.path(), &server);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let raw = std::fs::read_to_string(state_file(dir.path(), "catalogue.json"))
        .expect("catalogue.json written");
    assert!(!raw.contains("team-z-uuid"), "{raw}");
    assert!(!raw.contains("zed@example.com"), "{raw}");
}

#[test]
fn init_discover_writes_nothing_when_a_fetch_fails() {
    let server = MockServer::start();
    install(&server, "team-entries-failing-200");
    let dir = support::scratch(support::CONFIG);
    seed_catalogue(dir.path(), SYNCED_TEAM_Y);

    let output = discover(dir.path(), &server);

    assert!(!output.status.success());
    let raw = std::fs::read_to_string(state_file(dir.path(), "catalogue.json"))
        .expect("the catalogue is still there");
    assert_eq!(raw, SYNCED_TEAM_Y);
}

#[test]
fn init_discover_refuses_a_damaged_catalogue_naming_the_recovery() {
    let server = MockServer::start();
    install(&server, "team-states-200");
    let dir = support::scratch(support::CONFIG);
    let damaged = "<<<<<<< ours\n{}\n=======\n{}\n>>>>>>> theirs\n";
    seed_catalogue(dir.path(), damaged);

    let output = discover(dir.path(), &server);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let restore = stderr.find("version control").expect("restore named");
    let delete = stderr.find("delete").expect("deletion named");
    assert!(restore < delete, "{stderr}");
    let raw = std::fs::read_to_string(state_file(dir.path(), "catalogue.json"))
        .expect("the catalogue is still there");
    assert_eq!(raw, damaged);
    assert_eq!(
        server.hits(&http_test_support::RequestKey::post("/graphql")),
        0
    );
}

#[test]
fn init_discover_notes_a_legacy_file_without_teams() {
    let server = MockServer::start();
    install(&server, "team-states-200");
    let dir = support::scratch(support::CONFIG);
    seed_catalogue(
        dir.path(),
        r#"{"team": {"id": "team-x-uuid", "key": "BLA", "name": "Team X"},
            "workflowStates": []}"#,
    );

    let output = discover(dir.path(), &server);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let note = stderr.lines().find(|line| line.starts_with("note:"));
    assert!(note.is_some(), "no note: {stderr}");
    let note = note.unwrap();
    assert!(
        note.contains("If this repository synced other teams"),
        "{note}"
    );
    assert!(note.contains("older binary"), "{note}");
    assert!(note.contains("version control"), "{note}");
}
