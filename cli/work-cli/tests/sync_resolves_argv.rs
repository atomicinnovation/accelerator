//! Pins that the argv `/sync-work-items` emits is accepted by the real
//! `accelerator work sync`, network-free.
//!
//! A usage error (exit 2) would mean the invocation template is malformed —
//! the likeliest defect in a mostly-prose change. A valid `--resolve` run over
//! an empty corpus is any non-usage outcome; the two malformed shapes must be
//! exactly 2.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

mod common;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo(config: &str) -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-resolve-argv-")
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

fn run(
    dir: &Path,
    extra_env: &[(&str, &str)],
    args: &[&str],
) -> Result<std::process::Output, TestError> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .arg("sync")
        .arg("--preview")
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null());
    common::scrub_provider_env(&mut command);
    for (name, value) in extra_env {
        command.env(name, value);
    }
    Ok(command.output()?)
}

const JIRA_CONFIG: &str = "---\nwork:\n  integration: jira\n  \
    default_project_code: ENG\njira:\n  site: example\n  \
    email: t@e.x\n---\n";

#[test]
fn a_valid_resolve_argv_is_not_a_usage_error() -> Result<(), TestError> {
    let dir = scratch_repo(JIRA_CONFIG)?;
    let out = run(
        dir.path(),
        &[("ACCELERATOR_JIRA_TOKEN", "secret")],
        &["--resolve", "0001=remote", "--resolve", "0002=local"],
    )?;
    assert!(
        matches!(out.status.code(), Some(0 | 4 | 5 | 70 | 71 | 74)),
        "a valid argv must not be rejected; got {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_ne!(out.status.code(), Some(2));
    Ok(())
}

#[test]
fn creds_absent_reports_unconfigured() -> Result<(), TestError> {
    let dir = scratch_repo(JIRA_CONFIG)?;
    let out = run(dir.path(), &[], &["--resolve", "0001=remote"])?;
    assert_eq!(out.status.code(), Some(74));
    Ok(())
}

#[test]
fn a_repeated_id_is_a_usage_error() -> Result<(), TestError> {
    let dir = scratch_repo(JIRA_CONFIG)?;
    let out = run(
        dir.path(),
        &[("ACCELERATOR_JIRA_TOKEN", "secret")],
        &["--resolve", "0001=remote", "--resolve", "0001=local"],
    )?;
    assert_eq!(out.status.code(), Some(2));
    Ok(())
}

#[test]
fn a_value_with_no_equals_is_a_usage_error() -> Result<(), TestError> {
    let dir = scratch_repo(JIRA_CONFIG)?;
    let out = run(
        dir.path(),
        &[("ACCELERATOR_JIRA_TOKEN", "secret")],
        &["--resolve", "0001"],
    )?;
    assert_eq!(out.status.code(), Some(2));
    Ok(())
}
