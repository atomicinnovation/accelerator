//! The 138-row `vcs guard` decision table, replayed end to end through the
//! compiled `accelerator-vcs` binary over real jj/git checkouts.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use serde_json::Value;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-vcs");

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("vcs-guard-decision-{tag}-"))
        .tempdir()?)
}

fn decision_table() -> Result<Vec<Value>, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../hooks/test-fixtures/vcs-guard/decision-table.json");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}

fn pure_jj_repo(base: &Path, env: &Hermetic) -> Result<PathBuf, TestError> {
    let root = base.join("pure-jj");
    fs::create_dir_all(&root)?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    Ok(root)
}

fn colocated_repo(
    base: &Path,
    name: &str,
    env: &Hermetic,
) -> Result<PathBuf, TestError> {
    let root = base.join(name);
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "init"], &root)?;
    env.jj(&["git", "init", "--colocate"], &root)?;
    Ok(root)
}

fn git_only_repo(base: &Path, env: &Hermetic) -> Result<PathBuf, TestError> {
    let root = base.join("git");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "init"], &root)?;
    Ok(root)
}

fn non_repo(base: &Path) -> Result<PathBuf, TestError> {
    let root = base.join("non-repo");
    fs::create_dir_all(&root)?;
    Ok(root)
}

/// A colocated checkout whose `.git` has been replaced with a `gitdir:`
/// redirect file — the shape a linked worktree or submodule leaves behind.
fn colocated_git_as_file_repo(
    base: &Path,
    env: &Hermetic,
) -> Result<PathBuf, TestError> {
    let root = colocated_repo(base, "colocated-git-as-file", env)?;
    let real_git = root.join(".git-real");
    fs::rename(root.join(".git"), &real_git)?;
    fs::write(
        root.join(".git"),
        format!("gitdir: {}\n", real_git.display()),
    )?;
    Ok(root)
}

fn directory_for<'a>(
    directories: &'a [(&'static str, PathBuf)],
    repo_mode: &str,
) -> Result<&'a Path, TestError> {
    directories
        .iter()
        .find(|(name, _)| *name == repo_mode)
        .map(|(_, path)| path.as_path())
        .ok_or_else(|| {
            format!("no fixture directory for repo_mode {repo_mode}").into()
        })
}

/// Runs `accelerator-vcs vcs guard` from `root` with `command` as the tool
/// call's stdin payload, returning `(decision, reason)` normalised to the
/// decision table's own shape: `allow`/`block`/`warn`.
fn guard_decision(
    root: &Path,
    command: &str,
) -> Result<(String, String), TestError> {
    let mut child = Command::new(BIN)
        .current_dir(root)
        .arg("guard")
        .arg("--format=hook")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let payload =
        serde_json::json!({ "tool_input": { "command": command } }).to_string();
    child
        .stdin
        .take()
        .ok_or("no stdin")?
        .write_all(payload.as_bytes())?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "accelerator-vcs guard exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(("allow".to_owned(), String::new()));
    }
    let value: Value = serde_json::from_str(trimmed)?;
    if let Some(reason) = value
        .pointer("/hookSpecificOutput/permissionDecisionReason")
        .and_then(Value::as_str)
    {
        return Ok(("block".to_owned(), reason.to_owned()));
    }
    if let Some(message) = value.get("systemMessage").and_then(Value::as_str) {
        return Ok(("warn".to_owned(), message.to_owned()));
    }
    Err(format!("unrecognised guard envelope: {trimmed}").into())
}

#[test]
fn every_decision_table_row_matches() -> Result<(), TestError> {
    let work = tempdir("all")?;
    let env = Hermetic::rooted_at(work.path())?;

    let directories: Vec<(&'static str, PathBuf)> = vec![
        ("pure-jj", pure_jj_repo(work.path(), &env)?),
        ("colocated", colocated_repo(work.path(), "colocated", &env)?),
        ("git", git_only_repo(work.path(), &env)?),
        ("non-repo", non_repo(work.path())?),
        (
            "colocated-git-as-file",
            colocated_git_as_file_repo(work.path(), &env)?,
        ),
    ];

    let rows = decision_table()?;
    assert_eq!(rows.len(), 138, "expected 138 decision-table rows");

    for row in &rows {
        let repo_mode = row["repo_mode"].as_str().ok_or("missing repo_mode")?;
        let command = row["command"].as_str().ok_or("missing command")?;
        let expected_decision =
            row["decision"].as_str().ok_or("missing decision")?;
        let expected_reason = row["reason_pattern"].as_str().unwrap_or("");

        let root = directory_for(&directories, repo_mode)?;
        let (decision, reason) = guard_decision(root, command)?;

        assert_eq!(
            decision, expected_decision,
            "repo_mode: {repo_mode}, command: {command:?}"
        );
        assert_eq!(
            reason, expected_reason,
            "repo_mode: {repo_mode}, command: {command:?}"
        );
    }
    Ok(())
}
