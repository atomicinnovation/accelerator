//! CLI-boundary tests for `work sync`.
//!
//! `accelerator-work` is bin-only (no `[lib]` target, matching every other
//! `*-cli` crate — see `exit_codes_parity.rs`'s own note), so a real
//! subprocess invocation cannot inject a fake `TrackerRegistry`: the
//! compiled binary always resolves through `ConfiguredTrackers`, which has
//! no client wired for any provider yet. This suite therefore covers
//! exactly what is reachable from outside the process today — provider
//! selection, usage errors, and non-interactivity — and defers the
//! fake-tracker-driven scenarios (the two-invocation conflict loop,
//! classification stability, the write-bounds boundary tests) to a future
//! change that gives `work-cli` a `[lib]` target `work_adapters::sync::run`
//! can be driven through directly.
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

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
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .arg("sync")
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null())
        .output()?)
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
fn a_recognised_but_unbuilt_tracker_exits_72() -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(72));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("no client wired yet"), "{stderr}");
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
    assert_eq!(output.status.code(), Some(72));
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
    // A run that hung on stdin would make this test hang too; the process
    // completing at all (within the harness's own timeout) is the
    // assertion that matters here.
    assert_eq!(output.status.code(), Some(72));
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
    for code in ["0", "1", "2", "4", "5", "70", "71", "72", "73"] {
        assert!(
            stdout.contains(code),
            "help text is missing exit code {code}: {stdout}"
        );
    }
    Ok(())
}
