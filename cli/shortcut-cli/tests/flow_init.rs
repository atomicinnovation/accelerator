//! `init verify`, `init list-teams` and `init discover` end to end.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use serde_json::Value;

fn config_text(dir: &std::path::Path) -> String {
    std::fs::read_to_string(dir.join(".accelerator/config.md"))
        .expect("the team config")
}

#[test]
fn verify_reports_the_workspace_member_and_scopes() {
    let server = support::serving(&["whoami-200"]);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "verify"]);

    assert!(output.status.success(), "{}", support::stderr(&output));
    let stdout = support::stdout_json(&output);
    assert_eq!(stdout["outcome"], "verified");
    assert_eq!(stdout["workspace"]["slug"], "scratch-workspace");
    assert_eq!(stdout["member"]["mention_name"], "member-1");
    assert_eq!(stdout["read_only"], false);
}

#[test]
fn verify_names_a_read_only_token() {
    let server = support::serving(&["whoami-read-only-200"]);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["init", "verify"]);

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout_json(&output)["read_only"], true);
    assert!(support::stderr(&output).contains("read scope only"));
}

#[test]
fn list_teams_lists_every_team_by_its_key() {
    let server = support::serving(&["whoami-200", "workspace-200"]);
    let dir = support::scratch(support::KEYLESS_CONFIG);

    let output = support::run(dir.path(), &server, &["init", "list-teams"]);

    assert!(output.status.success(), "{}", support::stderr(&output));
    let stdout = support::stdout_json(&output);
    assert_eq!(stdout["outcome"], "listed");
    let keys: Vec<&str> = stdout["teams"]
        .as_array()
        .expect("a team list")
        .iter()
        .filter_map(|team| team["key"].as_str())
        .collect();
    assert_eq!(keys, ["team-1", "web"]);
}

#[test]
fn discover_writes_the_catalogue_and_an_absent_team_key() {
    let server = support::serving(&["whoami-200", "workspace-200"]);
    let dir = support::scratch(support::KEYLESS_CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["init", "discover", "--team", "team-1"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    let stdout = support::stdout_json(&output);
    assert_eq!(stdout["outcome"], "discovered");
    assert_eq!(stdout["team"]["key"], "team-1");
    let catalogue: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join(support::CATALOGUE))
            .expect("catalogue.json is written"),
    )
    .expect("the catalogue is JSON");
    assert_eq!(catalogue["workspace"]["slug"], "scratch-workspace");
    assert_eq!(catalogue["teams"].as_array().map(Vec::len), Some(2));
    let config = config_text(dir.path());
    assert!(config.contains("team_key: \"team-1\""), "{config}");
}

#[test]
fn discover_leaves_a_different_team_key_intact_without_force() {
    let server = support::serving(&["whoami-200", "workspace-200"]);
    let dir = support::scratch(
        "---\nwork:\n  integration: shortcut\nshortcut:\n  team_key: web\n---\n",
    );

    let output = support::run(
        dir.path(),
        &server,
        &["init", "discover", "--team", "team-1"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert!(
        support::stderr(&output).contains("shortcut.team_key left intact"),
        "{}",
        support::stderr(&output)
    );
    assert!(config_text(dir.path()).contains("team_key: web"));
}

#[test]
fn discover_with_force_overwrites_a_different_team_key() {
    let server = support::serving(&["whoami-200", "workspace-200"]);
    let dir = support::scratch(
        "---\nwork:\n  integration: shortcut\nshortcut:\n  team_key: web\n---\n",
    );

    let output = support::run(
        dir.path(),
        &server,
        &["init", "discover", "--team", "team-1", "--force"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    let config = config_text(dir.path());
    assert!(config.contains("team_key: \"team-1\""), "{config}");
}

#[test]
fn discover_for_an_unknown_team_writes_nothing() {
    let server = support::serving(&["whoami-200", "workspace-200"]);
    let dir = support::scratch(support::KEYLESS_CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["init", "discover", "--team", "nobody"],
    );

    assert_eq!(output.status.code(), Some(62));
    assert!(support::stderr(&output).contains("team-1"));
    assert!(!dir.path().join(support::CATALOGUE).exists());
}
