//! `vcs status`/`vcs log` against the committed golden fixtures, replayed end to
//! end through the compiled `accelerator-vcs` binary over the real jj/git
//! checkout states in `vcs-test-support/src/status_log.rs`, under
//! `vcs-test-support/fixtures/vcs-status-log`.
//!
//! Setting `REGENERATE_GOLDENS=1` rewrites the golden files from the current
//! binary output (masked) instead of comparing — the generation path the ADR
//! goldens are produced through, reviewed by hand against ADR-0066.
#![cfg(feature = "bash-parity")]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::masks;
use vcs_test_support::status_log;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-vcs");

fn masks_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../vcs-test-support/fixtures/masks.toml")
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../vcs-test-support/fixtures/vcs-status-log")
}

fn golden(name: &str) -> Result<String, TestError> {
    Ok(fs::read_to_string(
        fixtures_dir().join(format!("{name}.txt")),
    )?)
}

fn run_vcs(subcommand: &str, dir: &Path) -> Result<String, TestError> {
    let output = Command::new(BIN)
        .arg(subcommand)
        .arg("--fail-safe")
        .current_dir(dir)
        .output()?;
    assert!(
        output.status.success(),
        "accelerator-vcs {subcommand} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?
        .trim_end_matches('\n')
        .to_owned())
}

fn regenerating() -> bool {
    std::env::var_os("REGENERATE_GOLDENS").is_some()
}

#[test]
fn status_and_log_match_every_captured_state() -> Result<(), TestError> {
    let masks = masks::load(&masks_path())?;
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-goldens-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let states = status_log::build_states(work.path(), &env)?;

    let mut failures = Vec::new();
    for (name, directory) in &states {
        for subcommand in ["status", "log"] {
            let rendered =
                masks::apply(&masks, &run_vcs(subcommand, directory)?)?;
            let golden_name = format!("{name}-{subcommand}");
            if regenerating() {
                fs::write(
                    fixtures_dir().join(format!("{golden_name}.txt")),
                    format!("{}\n", rendered.trim_end_matches('\n')),
                )?;
                continue;
            }
            let expected = golden(&golden_name)?;
            if rendered.trim_end_matches('\n')
                != expected.trim_end_matches('\n')
            {
                failures.push(format!(
                    "{golden_name}:\n  expected: {expected:?}\n  \
                     actual:   {rendered:?}"
                ));
            }
        }
    }

    assert!(
        regenerating() || failures.is_empty(),
        "{}",
        failures.join("\n\n")
    );
    Ok(())
}

#[test]
fn a_conflict_status_carries_the_conflicted_marker_and_path(
) -> Result<(), TestError> {
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-conflict-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let states = status_log::build_states(work.path(), &env)?;

    for name in ["conflict-git", "conflict-jj"] {
        let directory = states.get(name).ok_or("conflict state missing")?;
        let rendered = run_vcs("status", directory)?;
        assert!(
            rendered.contains("conflicted") && rendered.contains("f.txt"),
            "{name} must mark the unmerged path conflicted: {rendered:?}"
        );
    }

    // jj's conflict is absent from the change diff and unioned in from the
    // tree, so it lists exactly the one conflicted path and no other change.
    let conflict_jj = states.get("conflict-jj").ok_or("conflict-jj missing")?;
    let rendered = run_vcs("status", conflict_jj)?;
    assert!(
        rendered.contains("1 changed, 1 conflicted"),
        "conflict-jj must show a single conflicted change: {rendered:?}"
    );
    Ok(())
}

#[test]
fn a_rename_renders_as_deleted_old_plus_added_new() -> Result<(), TestError> {
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-rename-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let states = status_log::build_states(work.path(), &env)?;
    let rename_git = states.get("rename-git").ok_or("rename-git missing")?;

    let rendered = run_vcs("status", rename_git)?;
    let body: Vec<&str> = rendered.lines().skip(2).collect();
    assert_eq!(
        body,
        ["  added  new.txt", "  deleted  old.txt"],
        "a staged rename is exactly deleted-old plus added-new: {rendered:?}"
    );
    Ok(())
}

#[test]
fn the_log_is_capped_at_five_entries() -> Result<(), TestError> {
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-cap-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let states = status_log::build_cap_states(work.path(), &env)?;

    for (name, directory) in &states {
        let log = run_vcs("log", directory)?;
        assert_eq!(
            log.lines().count(),
            5,
            "{name} log must be capped at five entries: {log:?}"
        );
        assert!(
            !log.contains("commit-1"),
            "{name} must omit the sixth ancestor: {log:?}"
        );
    }
    Ok(())
}

#[test]
fn an_empty_history_renders_no_commits() -> Result<(), TestError> {
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-empty-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let states = status_log::build_states(work.path(), &env)?;

    for name in ["unborn-git", "empty-jj"] {
        let directory = states.get(name).ok_or("empty state missing")?;
        assert_eq!(
            run_vcs("log", directory)?,
            "No commits",
            "{name} log must be No commits"
        );
    }
    let unborn = states.get("unborn-git").ok_or("unborn-git missing")?;
    assert!(
        run_vcs("status", unborn)?.contains("No changes"),
        "unborn-git status must be No changes"
    );
    Ok(())
}

#[test]
fn a_jj_bookmark_header_lists_the_byte_sorted_bookmarks(
) -> Result<(), TestError> {
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-bookmark-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let states = status_log::build_bookmark_states(work.path(), &env)?;

    let one = states.get("bookmark-one-jj").ok_or("one missing")?;
    assert!(
        run_vcs("status", one)?.starts_with("Branch: solo"),
        "a single bookmark must head the status"
    );
    let two = states.get("bookmark-two-jj").ok_or("two missing")?;
    assert!(
        run_vcs("status", two)?.starts_with("Branch: alpha, zed"),
        "two bookmarks must be byte-sorted and comma-joined"
    );
    Ok(())
}

#[test]
fn a_malformed_accelerator_log_still_renders_and_exits_zero(
) -> Result<(), TestError> {
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-badlog-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let mut states = BTreeMap::new();
    status_log::build_plain_git_states(work.path(), &env, &mut states)?;
    let root = states.get("clean-git").ok_or("clean-git state missing")?;

    for subcommand in ["status", "log"] {
        let output = Command::new(BIN)
            .arg(subcommand)
            .env("ACCELERATOR_LOG", "bad=notalevel")
            .current_dir(root)
            .output()?;
        assert!(
            output.status.success(),
            "{subcommand} must exit 0 under a malformed ACCELERATOR_LOG: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.stdout.is_empty(),
            "{subcommand} must still render under a malformed ACCELERATOR_LOG"
        );
    }
    Ok(())
}

#[test]
fn fail_safe_has_no_effect_on_a_successful_status_or_log(
) -> Result<(), TestError> {
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-fail-safe-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let mut states = BTreeMap::new();
    status_log::build_plain_git_states(work.path(), &env, &mut states)?;
    let root = states.get("clean-git").ok_or("clean-git state missing")?;

    for subcommand in ["status", "log"] {
        let with_flag = Command::new(BIN)
            .arg(subcommand)
            .arg("--fail-safe")
            .current_dir(root)
            .output()?;
        assert!(with_flag.status.success());

        let without_flag = Command::new(BIN)
            .arg(subcommand)
            .current_dir(root)
            .output()?;
        assert!(without_flag.status.success());

        assert_eq!(
            with_flag.stdout, without_flag.stdout,
            "--fail-safe must not change {subcommand}'s output on success"
        );
    }
    Ok(())
}
