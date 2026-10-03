//! Consent through the binary: the token command's refusals and notice on
//! stderr, and the frozen exit codes they map to.

#![cfg(all(feature = "test-loopback", unix))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;
use std::process::Output;

use cli_test_support::Scenario;
use http_test_support::{MockHTTPServer, RequestKey};

const TEAM_COMMAND: &str = "---\nwork:\n  integration: linear\nlinear:\n  \
    team_id: 5c9f2a1b-0000-4000-8000-000000000001\n  \
    token_cmd: printf team\n---\n";

fn showing_server() -> MockHTTPServer {
    let server = MockHTTPServer::start();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios/show-issue-200.json");
    Scenario::load(&path)
        .expect("a valid scenario")
        .install(&server);
    server
}

fn show(
    dir: &Path,
    server: &MockHTTPServer,
    environment: &[(&str, &str)],
) -> Output {
    support::run_without_token(dir, server, &["show", "BLA-42"], environment)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_team_token_command_alone_exits_no_token() {
    let server = showing_server();
    let dir = support::scratch(TEAM_COMMAND);

    let output = show(dir.path(), &server, &[]);

    assert_eq!(output.status.code(), Some(24), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("E_CONSENT_KEY_TEAM_LEVEL: linear.token_cmd"),
        "{}",
        stderr(&output)
    );
    assert_eq!(server.hits(&RequestKey::post("/graphql")), 0);
}

#[test]
fn a_team_token_command_beside_a_personal_token_warns_and_runs() {
    let server = showing_server();
    let dir = support::scratch(TEAM_COMMAND);
    support::write_personal_config(
        dir.path(),
        "---\nlinear:\n  token: lin_api_mine\n---\n",
        0o600,
    );

    let output = show(dir.path(), &server, &[]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output)
            .contains("warning: E_CONSENT_KEY_TEAM_LEVEL: linear.token_cmd"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_tracked_personal_command_beside_a_team_command_is_fatal_and_warns() {
    let server = showing_server();
    let dir = support::scratch(TEAM_COMMAND);
    support::write_personal_config(
        dir.path(),
        "---\nlinear:\n  token_cmd: printf mine\n---\n",
        0o600,
    );
    support::track_personal(dir.path());

    let output = show(dir.path(), &server, &[]);

    assert_eq!(output.status.code(), Some(24), "{}", stderr(&output));
    let stderr = stderr(&output);
    assert!(
        stderr.contains("warning: E_CONSENT_KEY_TEAM_LEVEL: linear.token_cmd"),
        "{stderr}"
    );
    assert!(
        stderr.contains("E_CONSENT_KEY_TRACKED: linear.token_cmd"),
        "{stderr}"
    );
}

#[test]
fn a_failed_token_command_exits_token_cmd_failed() {
    let server = showing_server();
    let dir = support::scratch(support::CONFIG);

    let output = show(
        dir.path(),
        &server,
        &[("ACCELERATOR_LINEAR_TOKEN_CMD", "exit 3")],
    );

    assert_eq!(output.status.code(), Some(25), "{}", stderr(&output));
    assert!(
        stderr(&output).contains(
            "E_TOKEN_CMD_FAILED: linear.token_cmd exited with status 3"
        ),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_token_carrying_a_control_character_exits_token_malformed() {
    let server = showing_server();
    let dir = support::scratch(support::CONFIG);

    let output = show(
        dir.path(),
        &server,
        &[("ACCELERATOR_LINEAR_TOKEN", "lin_api\u{1}x")],
    );

    assert_eq!(output.status.code(), Some(27), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("E_TOKEN_MALFORMED: linear.token"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_environment_token_command_is_noticed_without_its_command() {
    let server = showing_server();
    let dir = support::scratch(support::CONFIG);

    let output = show(
        dir.path(),
        &server,
        &[("ACCELERATOR_LINEAR_TOKEN_CMD", "printf lin_api_from_helper")],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains(
            "notice: linear.token_cmd taken from ACCELERATOR_LINEAR_TOKEN_CMD"
        ),
        "{}",
        stderr(&output)
    );
    assert!(!stderr(&output).contains("printf"), "{}", stderr(&output));
}

#[test]
fn an_insecure_personal_config_admits_an_environment_token_and_nothing_else() {
    let server = showing_server();
    let dir = support::scratch(support::CONFIG);
    support::write_personal_config(
        dir.path(),
        "---\nlinear:\n  token: lin_api_mine\n---\n",
        0o644,
    );

    let output = show(
        dir.path(),
        &server,
        &[("ACCELERATOR_LINEAR_TOKEN", "lin_api_env")],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("warning: E_LOCAL_PERMS_INSECURE"),
        "{}",
        stderr(&output)
    );

    let output = show(dir.path(), &server, &[]);
    assert_eq!(output.status.code(), Some(29), "{}", stderr(&output));
}

#[test]
fn a_personal_token_command_runs_outside_the_project_in_a_fresh_directory() {
    let server = showing_server();
    let dir = support::scratch(support::CONFIG);
    let record = tempfile::tempdir().unwrap();
    let recorded = record.path().join("cwd");
    support::write_personal_config(
        dir.path(),
        &format!(
            "---\nlinear:\n  token_cmd: pwd > {} && printf lin_api_cmd\n---\n",
            recorded.display()
        ),
        0o600,
    );

    let output = show(dir.path(), &server, &[]);

    assert!(output.status.success(), "{}", stderr(&output));
    let cwd = std::path::PathBuf::from(
        std::fs::read_to_string(&recorded).unwrap().trim(),
    );
    assert!(!cwd.starts_with(dir.path().canonicalize().unwrap()));
    assert!(!cwd.exists(), "{} outlived the run", cwd.display());
}
