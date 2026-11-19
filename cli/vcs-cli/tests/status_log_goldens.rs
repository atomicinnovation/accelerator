//! `vcs status`/`vcs log` against the committed golden fixtures, replayed end to
//! end through the compiled `accelerator-vcs` binary over the real jj/git
//! checkout states in `vcs-test-support/src/status_log.rs`, under
//! `vcs-test-support/fixtures/vcs-status-log`.
//!
//! Setting `REGENERATE_GOLDENS=1` rewrites the golden files from the current
//! binary output (masked) instead of comparing — the generation path the
//! goldens are produced through, reviewed by hand before they are committed.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::masks;
use vcs_test_support::status_log;
use vcs_test_support::status_log::StatusLogPart;
use vcs_test_support::workbench::Workbench;

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

fn run_vcs(
    env: &Hermetic,
    subcommand: &str,
    dir: &Path,
) -> Result<String, TestError> {
    let mut command = Command::new(BIN);
    env.apply(&mut command);
    let output = command
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

fn golden_names() -> Result<BTreeSet<String>, TestError> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(fixtures_dir())? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "txt") {
            let stem = path.file_stem().ok_or("a golden has no name")?;
            names.insert(stem.to_string_lossy().into_owned());
        }
    }
    Ok(names)
}

fn regenerating() -> bool {
    std::env::var_os("REGENERATE_GOLDENS").is_some()
}

#[test]
fn status_and_log_match_every_captured_state() -> Result<(), TestError> {
    let masks = masks::load(&masks_path())?;
    let workbench = Workbench::for_this_test()?;
    let states =
        workbench.writable_status_log_states(StatusLogPart::Everything)?;
    let env = workbench.hermetic();

    let mut failures = Vec::new();
    let mut compared = BTreeSet::new();
    for (name, directory) in states.with_goldens() {
        for subcommand in ["status", "log"] {
            let rendered =
                masks::apply(&masks, &run_vcs(env, subcommand, directory)?)?;
            let golden_name = format!("{name}-{subcommand}");
            compared.insert(golden_name.clone());
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
    assert_eq!(
        compared,
        golden_names()?,
        "every golden on disk is compared, and nothing else"
    );
    Ok(())
}

#[test]
fn a_conflict_status_carries_the_conflicted_marker_and_path(
) -> Result<(), TestError> {
    let workbench = Workbench::for_this_test()?;
    let states =
        workbench.writable_status_log_states(StatusLogPart::Everything)?;
    let env = workbench.hermetic();

    for name in ["conflict-git", "conflict-jj"] {
        let rendered = run_vcs(env, "status", states.get(name)?)?;
        assert!(
            rendered.contains("conflicted") && rendered.contains("f.txt"),
            "{name} must mark the unmerged path conflicted: {rendered:?}"
        );
    }

    // jj's conflict is absent from the change diff and unioned in from the
    // tree, so it lists exactly the one conflicted path and no other change.
    let rendered = run_vcs(env, "status", states.get("conflict-jj")?)?;
    assert!(
        rendered.contains("1 changed, 1 conflicted"),
        "conflict-jj must show a single conflicted change: {rendered:?}"
    );
    Ok(())
}

#[test]
fn a_rename_renders_as_deleted_old_plus_added_new() -> Result<(), TestError> {
    let workbench = Workbench::for_this_test()?;
    let states =
        workbench.writable_status_log_states(StatusLogPart::Everything)?;
    let env = workbench.hermetic();
    let rendered = run_vcs(env, "status", states.get("rename-git")?)?;
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
    let workbench = Workbench::for_this_test()?;
    let states = workbench.private_states(status_log::build_cap_states)?;
    let env = workbench.hermetic();

    for (name, directory) in states.iter() {
        let log = run_vcs(env, "log", directory)?;
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
    let workbench = Workbench::for_this_test()?;
    let states =
        workbench.writable_status_log_states(StatusLogPart::Everything)?;
    let env = workbench.hermetic();

    for name in ["unborn-git", "empty-jj"] {
        assert_eq!(
            run_vcs(env, "log", states.get(name)?)?,
            "No commits",
            "{name} log must be No commits"
        );
    }
    assert!(
        run_vcs(env, "status", states.get("unborn-git")?)?
            .contains("No changes"),
        "unborn-git status must be No changes"
    );
    Ok(())
}

#[test]
fn a_jj_bookmark_header_lists_the_byte_sorted_bookmarks(
) -> Result<(), TestError> {
    let workbench = Workbench::for_this_test()?;
    let states = workbench.private_states(status_log::build_bookmark_states)?;
    let env = workbench.hermetic();

    let one = states.get("bookmark-one-jj").ok_or("one missing")?;
    assert!(
        run_vcs(env, "status", one)?.starts_with("Branch: solo"),
        "a single bookmark must head the status"
    );
    let two = states.get("bookmark-two-jj").ok_or("two missing")?;
    assert!(
        run_vcs(env, "status", two)?.starts_with("Branch: alpha, zed"),
        "two bookmarks must be byte-sorted and comma-joined"
    );
    Ok(())
}

#[test]
fn a_malformed_accelerator_log_still_renders_and_exits_zero(
) -> Result<(), TestError> {
    let workbench = Workbench::for_this_test()?;
    let states =
        workbench.writable_status_log_states(StatusLogPart::PlainGit)?;
    let env = workbench.hermetic();
    let root = states.get("clean-git")?;

    for subcommand in ["status", "log"] {
        let mut command = Command::new(BIN);
        env.apply(&mut command);
        let output = command
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
fn a_forced_adapter_failure_emits_the_gix_token_to_stderr(
) -> Result<(), TestError> {
    // The deterministic D2 degenerate shape: a `.git` gitdir pointer to nowhere.
    // gix cannot open it, so status/log fall back — and with ACCELERATOR_LOG set
    // the warn must actually reach stderr, proving logging::init() delivers it
    // (it was discarded before this work) rather than the launcher's dead init.
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-token-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let repo = work.path().join("broken");
    fs::create_dir_all(&repo)?;
    fs::write(repo.join(".git"), "gitdir: /nonexistent/elsewhere\n")?;

    for subcommand in ["status", "log"] {
        let mut command = Command::new(BIN);
        env.apply(&mut command);
        let output = command
            .arg(subcommand)
            .env("ACCELERATOR_LOG", "warn")
            .current_dir(&repo)
            .output()?;
        assert!(
            output.status.success(),
            "{subcommand} must still exit 0 on an adapter failure"
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("unavailable"),
            "{subcommand} must render its fallback"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("gix"),
            "{subcommand} must warn-log the gix token: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn fail_safe_has_no_effect_on_a_successful_status_or_log(
) -> Result<(), TestError> {
    let workbench = Workbench::for_this_test()?;
    let states =
        workbench.writable_status_log_states(StatusLogPart::PlainGit)?;
    let env = workbench.hermetic();
    let root = states.get("clean-git")?;

    for subcommand in ["status", "log"] {
        let mut with_flag = Command::new(BIN);
        env.apply(&mut with_flag);
        let with_flag = with_flag
            .arg(subcommand)
            .arg("--fail-safe")
            .current_dir(root)
            .output()?;
        assert!(with_flag.status.success());

        let mut without_flag = Command::new(BIN);
        env.apply(&mut without_flag);
        let without_flag =
            without_flag.arg(subcommand).current_dir(root).output()?;
        assert!(without_flag.status.success());

        assert_eq!(
            with_flag.stdout, without_flag.stdout,
            "--fail-safe must not change {subcommand}'s output on success"
        );
    }
    Ok(())
}
