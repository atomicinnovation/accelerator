//! Consent through the binary: the allowlist's refusals and notice on stderr,
//! the frozen exit codes they map to, and an insecure personal config that
//! readers tolerate and writers refuse.

#![cfg(all(feature = "test-loopback", unix))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockHTTPServer, RequestKey, Route};

const MYSELF: &str = "/rest/api/3/myself";
const ISSUE: &str = "/rest/api/3/issue/ENG-42";
const CREATE: &str = "/rest/api/3/issue";
const COMMENT: &str = "/rest/api/3/issue/ENG-1/comment";
const SEARCH: &str = "/rest/api/3/search/jql";
const OUTSIDE: &str = "jira.example.com";

fn config(site: &str, allowlist: Option<&str>) -> String {
    let allowlist = allowlist
        .map_or(String::new(), |list| format!("  allowed_sites: {list}\n"));
    format!(
        "---\nwork:\n  integration: jira\njira:\n  site: {site}\n  \
         email: toby@example.com\n  project_key: ENG\n{allowlist}---\n"
    )
}

fn verifying_server() -> MockHTTPServer {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(MYSELF),
        Route::Json {
            status: 200,
            body: r#"{"accountId":"acc-1","displayName":"Ada"}"#.to_owned(),
        },
    );
    server
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_team_allowlist_beside_an_atlassian_site_warns_and_runs() {
    let server = verifying_server();
    let dir = support::scratch(&config("acme", Some(OUTSIDE)));

    let output = support::run(dir.path(), &server, &["init", "verify"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output)
            .contains("warning: E_CONSENT_KEY_TEAM_LEVEL: jira.allowed_sites"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_team_allowlist_beside_an_outside_site_exits_no_token() {
    let server = verifying_server();
    let dir =
        support::scratch(&config(&format!("https://{OUTSIDE}"), Some(OUTSIDE)));

    let output = support::run(dir.path(), &server, &["init", "verify"]);

    assert_eq!(output.status.code(), Some(24), "{}", stderr(&output));
    assert!(
        stderr(&output)
            .contains("E_CONSENT_KEY_TEAM_LEVEL: jira.allowed_sites"),
        "{}",
        stderr(&output)
    );
    assert_eq!(server.hits(&RequestKey::get(MYSELF)), 0);
}

#[test]
fn the_allowlist_override_is_noticed_with_its_value() {
    let server = verifying_server();
    let dir = support::scratch(&config(&format!("https://{OUTSIDE}"), None));

    let output = support::run_env(
        dir.path(),
        &server,
        &["init", "verify"],
        &[("ACCELERATOR_JIRA_ALLOWED_SITES", OUTSIDE)],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains(&format!(
            "notice: jira.allowed_sites taken from \
             ACCELERATOR_JIRA_ALLOWED_SITES: {OUTSIDE}"
        )),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_personal_allowlist_is_not_noticed() {
    let server = verifying_server();
    let dir = support::scratch(&config(&format!("https://{OUTSIDE}"), None));
    support::write_personal_config(
        dir.path(),
        &format!("---\njira:\n  allowed_sites: {OUTSIDE}\n---\n"),
        0o600,
    );

    let output = support::run(dir.path(), &server, &["init", "verify"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!stderr(&output).contains("notice:"), "{}", stderr(&output));
}

fn insecure_scratch() -> tempfile::TempDir {
    let dir = support::scratch(support::CONFIG);
    support::write_personal_config(
        dir.path(),
        "---\njira:\n  project_key: MINE\n---\n",
        0o644,
    );
    dir
}

#[test]
fn a_reader_runs_on_team_values_beside_an_insecure_personal_config() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(ISSUE),
        Route::Json {
            status: 200,
            body: r#"{"key":"ENG-42","fields":{"summary":"a bug"}}"#.to_owned(),
        },
    );
    let dir = insecure_scratch();

    let output = support::run(dir.path(), &server, &["show", "ENG-42"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stderr(&output)
            .matches("warning: E_LOCAL_PERMS_INSECURE")
            .count(),
        1,
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_search_that_composes_twice_warns_once() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: r#"{"issues":[],"isLast":true}"#.to_owned(),
        },
    );
    let dir = insecure_scratch();

    let output = support::run(
        dir.path(),
        &server,
        &["search", "--text", "bug", "--quiet"],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stderr(&output)
            .matches("warning: E_LOCAL_PERMS_INSECURE")
            .count(),
        1,
        "{}",
        stderr(&output)
    );
}

#[test]
fn writers_refuse_beside_an_insecure_personal_config_and_write_nothing() {
    let cases: [(&[&str], RequestKey, i32); 2] = [
        (
            &["create", "--summary", "x", "--type", "bug"],
            RequestKey::post(CREATE),
            1,
        ),
        (
            &["comment", "add", "ENG-1", "--body", "hi"],
            RequestKey::post(COMMENT),
            1,
        ),
    ];
    for (args, request, code) in cases {
        let server = MockHTTPServer::start();
        server.route(request.clone(), Route::Status(201));
        let dir = insecure_scratch();

        let output = support::run(dir.path(), &server, args);

        assert_eq!(
            output.status.code(),
            Some(code),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("E_LOCAL_PERMS_INSECURE"),
            "{args:?}: {}",
            stderr(&output)
        );
        assert_eq!(server.hits(&request), 0, "{args:?} wrote to the tracker");
    }
}

#[test]
fn resolve_fields_refuses_to_take_its_project_from_config() {
    let dir = insecure_scratch();

    let output = support::run_with(
        dir.path(),
        &["resolve-fields", "--kind", "story"],
        None,
        &support::Token::Present,
    );

    assert_eq!(output.status.code(), Some(108), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("E_LOCAL_PERMS_INSECURE"),
        "{}",
        stderr(&output)
    );
    assert!(output.stdout.is_empty(), "nothing is resolved");
}

fn token_cmd_config(team_token_cmd: Option<&str>) -> String {
    let command = team_token_cmd
        .map_or(String::new(), |command| format!("  token_cmd: {command}\n"));
    format!(
        "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
         email: toby@example.com\n  project_key: ENG\n{command}---\n"
    )
}

fn verify_without_token(
    dir: &std::path::Path,
    server: &MockHTTPServer,
    environment: &[(&str, &str)],
) -> std::process::Output {
    support::run_env_with(
        dir,
        server,
        &["init", "verify"],
        environment,
        &support::Token::Absent,
    )
}

#[test]
fn a_team_token_command_with_nothing_usable_exits_no_token() {
    let server = verifying_server();
    let dir = support::scratch(&token_cmd_config(Some("printf team")));

    let output = verify_without_token(dir.path(), &server, &[]);

    assert_eq!(output.status.code(), Some(24), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("E_CONSENT_KEY_TEAM_LEVEL: jira.token_cmd"),
        "{}",
        stderr(&output)
    );
    assert_eq!(server.hits(&RequestKey::get(MYSELF)), 0);
}

#[test]
fn a_team_token_command_beside_a_personal_token_warns_and_runs() {
    let server = verifying_server();
    let dir = support::scratch(&token_cmd_config(Some("printf team")));
    support::write_personal_config(
        dir.path(),
        "---\njira:\n  token: mine\n---\n",
        0o600,
    );

    let output = verify_without_token(dir.path(), &server, &[]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output)
            .contains("warning: E_CONSENT_KEY_TEAM_LEVEL: jira.token_cmd"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_environment_token_command_is_noticed_without_its_command() {
    let server = verifying_server();
    let dir = support::scratch(&token_cmd_config(None));

    let output = verify_without_token(
        dir.path(),
        &server,
        &[("ACCELERATOR_JIRA_TOKEN_CMD", "printf from-the-helper")],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains(
            "notice: jira.token_cmd taken from ACCELERATOR_JIRA_TOKEN_CMD"
        ),
        "{}",
        stderr(&output)
    );
    assert!(!stderr(&output).contains("printf"), "{}", stderr(&output));
}

fn showing_server() -> MockHTTPServer {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(ISSUE),
        Route::Json {
            status: 200,
            body: r#"{"key":"ENG-42","fields":{"summary":"a bug"}}"#.to_owned(),
        },
    );
    server
}

fn show_without_token(
    dir: &std::path::Path,
    server: &MockHTTPServer,
    environment: &[(&str, &str)],
) -> std::process::Output {
    support::run_env_with(
        dir,
        server,
        &["show", "ENG-42"],
        environment,
        &support::Token::Absent,
    )
}

#[test]
fn an_environment_token_command_runs_beside_an_insecure_personal_config() {
    let server = showing_server();
    let dir = insecure_scratch();

    let output = show_without_token(
        dir.path(),
        &server,
        &[("ACCELERATOR_JIRA_TOKEN_CMD", "printf from-the-helper")],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stderr(&output)
            .matches("warning: E_LOCAL_PERMS_INSECURE")
            .count(),
        1,
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_insecure_personal_config_with_nothing_usable_exits_perms_insecure() {
    let server = showing_server();
    let dir = insecure_scratch();

    let output = show_without_token(dir.path(), &server, &[]);

    assert_eq!(output.status.code(), Some(29), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("E_LOCAL_PERMS_INSECURE"),
        "{}",
        stderr(&output)
    );
    assert_eq!(server.hits(&RequestKey::get(ISSUE)), 0);
}

#[test]
fn a_tracked_personal_command_beside_a_team_command_is_fatal_and_warns() {
    let server = verifying_server();
    let dir = support::scratch(&token_cmd_config(Some("printf team")));
    support::write_personal_config(
        dir.path(),
        "---\njira:\n  token_cmd: printf mine\n---\n",
        0o600,
    );
    support::track_personal(dir.path());

    let output = verify_without_token(dir.path(), &server, &[]);

    assert_eq!(output.status.code(), Some(24), "{}", stderr(&output));
    let stderr = stderr(&output);
    assert!(
        stderr.contains("warning: E_CONSENT_KEY_TEAM_LEVEL: jira.token_cmd"),
        "{stderr}"
    );
    assert!(
        stderr.contains("E_CONSENT_KEY_TRACKED: jira.token_cmd"),
        "{stderr}"
    );
}

#[test]
fn a_helper_printing_a_control_character_exits_no_token() {
    let server = verifying_server();
    let dir = support::scratch(&token_cmd_config(None));

    let output = verify_without_token(
        dir.path(),
        &server,
        &[("ACCELERATOR_JIRA_TOKEN_CMD", "printf 'tok\\001'")],
    );

    assert_eq!(output.status.code(), Some(24), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("E_TOKEN_MALFORMED: jira.token_cmd"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_personal_token_command_runs_outside_the_project_in_a_fresh_directory() {
    let server = verifying_server();
    let dir = support::scratch(&token_cmd_config(None));
    let record = tempfile::tempdir().unwrap();
    let recorded = record.path().join("cwd");
    support::write_personal_config(
        dir.path(),
        &format!(
            "---\njira:\n  token_cmd: pwd > {} && printf tok\n---\n",
            recorded.display()
        ),
        0o600,
    );

    let output = verify_without_token(dir.path(), &server, &[]);

    assert!(output.status.success(), "{}", stderr(&output));
    let cwd = std::path::PathBuf::from(
        std::fs::read_to_string(&recorded).unwrap().trim(),
    );
    let project = dir.path().canonicalize().unwrap();
    assert!(!cwd.starts_with(&project), "{}", cwd.display());
    assert!(!cwd.exists(), "{} outlived the run", cwd.display());
}
