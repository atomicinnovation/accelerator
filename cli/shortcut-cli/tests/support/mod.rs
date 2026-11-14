//! Runs `accelerator-shortcut` as a subprocess against a mock, through the
//! `test-loopback` base-URL seam.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;

use cli_test_support::Scenario;
use http_test_support::MockHTTPServer;

pub const CONFIG: &str = "---\nwork:\n  integration: shortcut\nshortcut:\n  \
    team_key: team-1\n---\n";

pub const KEYLESS_CONFIG: &str = "---\nwork:\n  integration: shortcut\n---\n";

pub const TOKEN_SENTINEL: &str = "sct_rw_sentinel_do_not_leak";

pub const CATALOGUE: &str =
    ".accelerator/state/integrations/shortcut/catalogue.json";

pub fn scratch(config: &str) -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix("shortcut-cli-")
        .tempdir()
        .expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".accelerator"))
        .expect("mkdir .accelerator");
    std::fs::write(dir.path().join(".accelerator/config.md"), config)
        .expect("write config");
    dir
}

/// A catalogue for the workspace the committed scenarios answer as.
pub fn seed_catalogue(dir: &Path) {
    let catalogue = dir.join(CATALOGUE);
    std::fs::create_dir_all(catalogue.parent().expect("catalogue parent"))
        .expect("mkdir catalogue dir");
    std::fs::write(
        &catalogue,
        r#"{"schema_version": 1,
           "workspace": {"slug": "scratch-workspace", "name": "Scratch Workspace"},
           "teams": [{"id": "00000000-0000-4000-8000-000000000002",
                      "name": "Team 1", "mention_name": "team-1",
                      "default_workflow_id": 500000005,
                      "workflow_ids": [500000005]}],
           "workflows": [{"id": 500000005, "name": "Standard", "states": [
               {"id": 500000007, "name": "To Do", "type": "unstarted",
                "position": 1}]}]}"#,
    )
    .expect("write catalogue");
}

/// The same catalogue with a second "To Do" in the story's workflow.
pub fn seed_ambiguous_catalogue(dir: &Path) {
    seed_catalogue(dir);
    let path = dir.join(CATALOGUE);
    let text = std::fs::read_to_string(&path).expect("read catalogue");
    let doubled = text.replace(
        r#""position": 1}]}]}"#,
        r#""position": 1},
               {"id": 500000008, "name": "To do", "type": "started",
                "position": 2}]}]}"#,
    );
    assert_ne!(doubled, text, "the seed catalogue changed shape");
    std::fs::write(&path, doubled).expect("write catalogue");
}

pub fn sent_json(
    server: &MockHTTPServer,
    key: &http_test_support::RequestKey,
) -> serde_json::Value {
    let body = server.last_body(key).expect("a request was sent");
    serde_json::from_slice(&body).expect("the body is JSON")
}

pub fn scenario(name: &str) -> Scenario {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/scenarios")
        .join(format!("{name}.json"));
    Scenario::load(&path).expect("a valid scenario")
}

pub fn serving(names: &[&str]) -> MockHTTPServer {
    let server = MockHTTPServer::start();
    for name in names {
        scenario(name).install(&server);
    }
    server
}

pub enum Token {
    Present,
    Absent,
    Is(&'static str),
}

pub fn run(dir: &Path, server: &MockHTTPServer, args: &[&str]) -> Output {
    run_with(dir, args, Some(&server.base_url()), &Token::Present)
}

pub fn run_with(
    dir: &Path,
    args: &[&str],
    api_url: Option<&str>,
    token: &Token,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-shortcut"));
    command
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .env_remove("ACCELERATOR_SHORTCUT_TOKEN_CMD")
        .stdin(Stdio::null());
    match token {
        Token::Present => {
            command.env("ACCELERATOR_SHORTCUT_TOKEN", TOKEN_SENTINEL);
        }
        Token::Is(token) => {
            command.env("ACCELERATOR_SHORTCUT_TOKEN", token);
        }
        Token::Absent => {
            command.env_remove("ACCELERATOR_SHORTCUT_TOKEN");
        }
    }
    match api_url {
        Some(url) => command.env("ACCELERATOR_SHORTCUT_API_URL", url),
        None => command.env_remove("ACCELERATOR_SHORTCUT_API_URL"),
    };
    command.output().expect("run accelerator-shortcut")
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn stdout_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not one JSON document ({error}): {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            stderr(output)
        )
    })
}
