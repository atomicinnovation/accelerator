//! CLI-boundary tests for `work sync`.
//!
//! `accelerator-work` is bin-only, so a subprocess cannot inject a fake
//! `TrackerRegistry` and this suite covers only what is reachable from
//! outside the process: provider selection, usage errors, and
//! non-interactivity. The fake-tracker scenarios — the conflict loop,
//! classification stability, the write-bounds boundaries — need a `[lib]`
//! target `work_adapters::sync::run` can be driven through directly.
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

mod common;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo(
    integration: Option<&str>,
) -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-sync-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    if let Some(integration) = integration {
        fs::write(
            dir.path().join(".accelerator/config.md"),
            format!("---\nwork:\n  integration: {integration}\n---\n"),
        )?;
    }
    Ok(dir)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .arg("sync")
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null());
    // A credentialed machine must not resolve a real client and reach the
    // network from the default suite (Phase 10's network-free goal).
    common::scrub_provider_env(&mut command);
    Ok(command.output()?)
}

#[test]
fn unset_work_integration_exits_73_naming_the_key_and_configure(
) -> Result<(), TestError> {
    let repo = scratch_repo(None)?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(73));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("work.integration"), "{stderr}");
    assert!(stderr.contains("/accelerator:configure"), "{stderr}");
    Ok(())
}

#[test]
fn an_unrecognised_tracker_exits_73_and_echoes_the_value(
) -> Result<(), TestError> {
    let repo = scratch_repo(Some("bogus-tracker"))?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(73));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("bogus-tracker"), "{stderr}");
    Ok(())
}

#[test]
fn a_wired_tracker_without_credentials_exits_74() -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(74));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("not usable"), "{stderr}");
    assert!(stderr.contains("jira.site"), "{stderr}");
    Ok(())
}

#[test]
fn trello_also_exits_72_not_73() -> Result<(), TestError> {
    let repo = scratch_repo(Some("trello"))?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(72));
    Ok(())
}

#[test]
fn preview_does_not_bypass_provider_selection() -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &["--preview"])?;
    assert_eq!(output.status.code(), Some(74));
    Ok(())
}

#[test]
fn push_only_and_pull_only_together_is_a_usage_error() -> Result<(), TestError>
{
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &["--push-only", "--pull-only"])?;
    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[test]
fn a_malformed_resolve_order_is_a_usage_error() -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &["--resolve", "no-equals-sign"])?;
    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[test]
fn a_duplicate_resolve_order_for_one_id_is_a_usage_error(
) -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(
        repo.path(),
        &["--resolve", "0001=remote", "--resolve", "0001=local"],
    )?;
    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[test]
fn stdin_closed_never_blocks_and_never_reads_it() -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &[])?;
    // The process completing at all is the assertion: a run that hung on
    // stdin would hang this test.
    assert_eq!(output.status.code(), Some(74));
    Ok(())
}

#[test]
fn the_scrubbed_set_is_derived_from_the_token_ladders() -> Result<(), TestError>
{
    let scrubbed = common::provider_env_vars();
    for keys in [
        jira_client::auth::token_keys()?,
        linear_client::auth::token_keys()?,
    ] {
        assert!(scrubbed.contains(&keys.env.to_owned()), "{scrubbed:?}");
        assert!(
            scrubbed.contains(&keys.env_command.to_owned()),
            "{scrubbed:?}"
        );
    }
    assert!(
        scrubbed.contains(&"ACCELERATOR_ALLOW_INSECURE_LOCAL".to_owned()),
        "the insecure-local override must be scrubbed too: {scrubbed:?}"
    );
    assert_eq!(
        scrubbed.len(),
        5,
        "two token env names per provider plus the insecure marker"
    );
    Ok(())
}

#[test]
fn sync_help_names_every_exit_code() -> Result<(), TestError> {
    let repo = scratch_repo(None)?;
    let output = Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(["sync", "--help"])
        .current_dir(repo.path())
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    for code in ["0", "1", "2", "4", "5", "70", "71", "72", "73", "74"] {
        assert!(
            stdout.contains(code),
            "help text is missing exit code {code}: {stdout}"
        );
    }
    Ok(())
}
