//! `accelerator work sync` resolving a real client through
//! `ConfiguredTrackers`, network-free.
//!
//! `accelerator-work` is bin-only, and `from_config` refuses a loopback base
//! (that admission is a constructor parameter only), so a subprocess cannot
//! point a resolved client at a mock. What it *can* observe is the resolution
//! boundary: with credentials present and an empty corpus, resolution succeeds
//! and the empty fetch makes no network call, so the run exits 0; with the
//! token scrubbed, the same configuration reports `Unconfigured` and exits 74.
//! The end-to-end classification against a mock lives in
//! `work-adapters/tests/sync_run_real_client.rs`, where a client can be
//! constructed directly.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

mod common;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo(config: &str) -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-resolve-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(dir.path().join(".accelerator/config.md"), config)?;
    Ok(dir)
}

/// Runs `sync` with the provider environment scrubbed, then applies `extra`
/// (so a test can add exactly the one token it wants present).
fn run(
    dir: &Path,
    extra: &[(&str, &str)],
) -> Result<std::process::Output, TestError> {
    run_with_args(dir, &[], extra)
}

/// As [`run`], with additional `sync` flags.
fn run_with_args(
    dir: &Path,
    args: &[&str],
    extra: &[(&str, &str)],
) -> Result<std::process::Output, TestError> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .arg("sync")
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null());
    common::scrub_provider_env(&mut command);
    for (name, value) in extra {
        command.env(name, value);
    }
    Ok(command.output()?)
}

const JIRA_CONFIG: &str = "---\nwork:\n  integration: jira\n  \
    default_project_code: ENG\njira:\n  site: example\n  \
    email: t@e.x\n---\n";

const LINEAR_CONFIG: &str = "---\nwork:\n  integration: linear\nlinear:\n  \
    team_id: 5c9f2a1b-0000-4000-8000-000000000001\n---\n";

#[test]
fn jira_resolves_with_credentials_and_an_empty_corpus_exits_0(
) -> Result<(), TestError> {
    // `--push-only` disables untracked-remote discovery, which a Jira config
    // (always scoped to a project) would otherwise perform even on an empty
    // corpus. Without it the resolution boundary is what this asserts: an
    // empty push makes no network call, so the run exits 0.
    let repo = scratch_repo(JIRA_CONFIG)?;
    let output = run_with_args(
        repo.path(),
        &["--push-only"],
        &[("ACCELERATOR_JIRA_TOKEN", "dummy")],
    )?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "resolution succeeded and the empty push made no network call: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn jira_without_a_token_reports_unconfigured() -> Result<(), TestError> {
    let repo = scratch_repo(JIRA_CONFIG)?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(74));
    Ok(())
}

#[test]
fn linear_push_only_resolves_with_credentials_and_an_empty_corpus_exits_0(
) -> Result<(), TestError> {
    // `--push-only` skips discovery, so an unkeyed Linear config still exits 0:
    // resolution succeeds and the empty push makes no network call.
    let repo = scratch_repo(LINEAR_CONFIG)?;
    let output = run_with_args(
        repo.path(),
        &["--push-only"],
        &[("ACCELERATOR_LINEAR_TOKEN", "dummy")],
    )?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "resolution succeeded and the empty push made no network call: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn linear_bidirectional_without_a_key_refuses_discovery_and_exits_74(
) -> Result<(), TestError> {
    // A bidirectional run needs a discovery scope. An unkeyed Linear config
    // names no team, so the run is refused pre-flight — nothing sent, exit 74 —
    // rather than searching whatever team the credential happens to reach.
    let repo = scratch_repo(LINEAR_CONFIG)?;
    let output = run(repo.path(), &[("ACCELERATOR_LINEAR_TOKEN", "dummy")])?;
    assert_eq!(
        output.status.code(),
        Some(74),
        "an unkeyed bidirectional run refuses: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("discovery is unconfigured"),
        "the refusal names the discovery fault: {stderr}"
    );
    assert!(
        stderr.contains("work.default_project_code"),
        "the refusal names the missing key: {stderr}"
    );
    Ok(())
}

#[test]
fn linear_without_a_token_reports_unconfigured() -> Result<(), TestError> {
    let repo = scratch_repo(LINEAR_CONFIG)?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(74));
    Ok(())
}
