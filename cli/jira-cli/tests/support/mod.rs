//! Runs `accelerator-jira` as a subprocess against a mock, through the
//! `test-loopback` base-URL seam. Only compiled into tests gated on that
//! feature, so an ordinary build never carries the loopback-admitting path.

#![allow(dead_code, clippy::expect_used)]

use std::path::Path;
use std::process::{Command, Output, Stdio};

use http_test_support::MockHTTPServer;

/// The default config: jira integration, a site and email so credentials
/// resolve from an env token, and a Jira project key the resolver reads.
pub const CONFIG: &str = "---\nwork:\n  integration: jira\njira:\n  \
    site: acme\n  email: toby@example.com\n  project_key: ENG\n---\n";

pub const TOKEN_SENTINEL: &str = "jira_api_sentinel_do_not_leak";

/// A scratch repository with the given `.accelerator/config.md`.
pub fn scratch(config: &str) -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix("jira-cli-")
        .tempdir()
        .expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".accelerator"))
        .expect("mkdir .accelerator");
    std::fs::write(dir.path().join(".accelerator/config.md"), config)
        .expect("write config");
    dir
}

/// Seeds a markerless cache file under the Jira state dir, the shape `init`
/// writes and `create`/`update` read for `@me` and custom-field resolution.
pub fn seed_cache(dir: &Path, name: &str, content: &str) {
    let state = dir.join(".accelerator/state/integrations/jira");
    std::fs::create_dir_all(&state).expect("mkdir state dir");
    std::fs::write(state.join(name), content).expect("write cache");
}

/// Whether the binary run carries a resolvable credential.
pub enum Token {
    Present,
    Absent,
}

/// Asserts `actual` byte-for-byte against the committed golden at `name` under
/// `tests/fixtures/goldens/`. Running with `UPDATE_GOLDEN=1` rewrites the golden
/// instead of asserting, so a deliberate output change is a reviewable diff.
pub fn assert_golden(name: &str, actual: &[u8]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/goldens")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().expect("golden parent"))
            .expect("mkdir goldens");
        std::fs::write(&path, actual).expect("write golden");
        return;
    }
    let expected = std::fs::read(&path)
        .expect("missing golden — run with UPDATE_GOLDEN=1 to create it");
    assert_eq!(
        actual,
        expected.as_slice(),
        "stdout diverged from golden {}; re-run with UPDATE_GOLDEN=1 to accept",
        path.display()
    );
}

/// Runs the binary against `server` from `dir`, with the env token and the
/// loopback API URL (the mock root) set and the token-command env scrubbed.
pub fn run(dir: &Path, server: &MockHTTPServer, args: &[&str]) -> Output {
    run_with(dir, args, Some(&server.base_url()), &Token::Present)
}

/// As [`run`], with extra environment variables set.
pub fn run_env(
    dir: &Path,
    server: &MockHTTPServer,
    args: &[&str],
    environment: &[(&str, &str)],
) -> Output {
    let mut command =
        command(dir, args, Some(&server.base_url()), &Token::Present);
    command.envs(environment.iter().copied());
    command.output().expect("run accelerator-jira")
}

/// As [`run_env`], with the env token present or absent.
pub fn run_env_with(
    dir: &Path,
    server: &MockHTTPServer,
    args: &[&str],
    environment: &[(&str, &str)],
    token: &Token,
) -> Output {
    let mut command = command(dir, args, Some(&server.base_url()), token);
    command.envs(environment.iter().copied());
    command.output().expect("run accelerator-jira")
}

/// Makes `dir` a git repository whose index holds its
/// `.accelerator/config.local.md`, so the personal file reads as tracked.
pub fn track_personal(dir: &Path) {
    for args in [
        &["init", "--quiet"][..],
        &["add", "--force", ".accelerator/config.local.md"][..],
    ] {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?} failed");
    }
}

/// Writes `.accelerator/config.local.md` at `mode`.
#[cfg(unix)]
pub fn personal(dir: &Path, content: &str, mode: u32) {
    use std::os::unix::fs::PermissionsExt as _;

    let path = dir.join(".accelerator/config.local.md");
    std::fs::write(&path, content).expect("write the personal config");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
        .expect("chmod the personal config");
}

/// Runs the binary with an explicit `ACCELERATOR_JIRA_API_URL` (or none) and a
/// present or absent token — the seam and missing-credential paths.
pub fn run_with(
    dir: &Path,
    args: &[&str],
    api_url: Option<&str>,
    token: &Token,
) -> Output {
    command(dir, args, api_url, token)
        .output()
        .expect("run accelerator-jira")
}

fn command(
    dir: &Path,
    args: &[&str],
    api_url: Option<&str>,
    token: &Token,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-jira"));
    command
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .env_remove("ACCELERATOR_JIRA_TOKEN_CMD")
        .stdin(Stdio::null());
    match token {
        Token::Present => {
            command.env("ACCELERATOR_JIRA_TOKEN", TOKEN_SENTINEL);
        }
        Token::Absent => {
            command.env_remove("ACCELERATOR_JIRA_TOKEN");
        }
    }
    match api_url {
        Some(url) => {
            command.env("ACCELERATOR_JIRA_API_URL", url);
        }
        None => {
            command.env_remove("ACCELERATOR_JIRA_API_URL");
        }
    }
    command.env_remove("ACCELERATOR_JIRA_ALLOWED_SITES");
    command
}
