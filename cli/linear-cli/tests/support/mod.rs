//! Runs `accelerator-linear` as a subprocess against a mock, through the
//! `test-loopback` base-URL seam. Only compiled into tests gated on that
//! feature, so an ordinary build never carries the loopback-admitting path.

#![allow(dead_code, clippy::expect_used)]

use std::path::Path;
use std::process::{Command, Output, Stdio};

use http_test_support::MockServer;

/// The default config: linear integration plus a team id, so credentials
/// resolve from an env token without a catalogue on disk.
pub const CONFIG: &str = "---\nwork:\n  integration: linear\nlinear:\n  \
    team_id: 5c9f2a1b-0000-4000-8000-000000000001\n---\n";

pub const TOKEN_SENTINEL: &str = "lin_api_sentinel_do_not_leak";

/// A scratch repository with the given `.accelerator/config.md`.
pub fn scratch(config: &str) -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix("linear-cli-")
        .tempdir()
        .expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".accelerator"))
        .expect("mkdir .accelerator");
    std::fs::write(dir.path().join(".accelerator/config.md"), config)
        .expect("write config");
    dir
}

/// Seeds the workflow-state catalogue `resolve_state` reads, at the default
/// `paths.integrations` location, so a transition can resolve a name to a UUID.
pub fn seed_catalogue(dir: &Path) {
    let catalogue =
        dir.join(".accelerator/state/integrations/linear/catalogue.json");
    std::fs::create_dir_all(catalogue.parent().expect("catalogue parent"))
        .expect("mkdir catalogue dir");
    std::fs::write(
        &catalogue,
        r#"{"team": {"key": "BLA", "id": "team-uuid"},
           "workflowStates": [{"name": "In Progress", "id": "state-ip-uuid"}]}"#,
    )
    .expect("write catalogue");
}

/// Runs the binary against `server` from `dir`, with the env token and the
/// loopback API URL set and the token-command env scrubbed.
pub fn run(dir: &Path, server: &MockServer, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-linear"));
    command
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .env("ACCELERATOR_LINEAR_TOKEN", TOKEN_SENTINEL)
        .env(
            "ACCELERATOR_LINEAR_API_URL",
            format!("{}/graphql", server.base_url()),
        )
        .env_remove("ACCELERATOR_LINEAR_TOKEN_CMD")
        .stdin(Stdio::null());
    command.output().expect("run accelerator-linear")
}
