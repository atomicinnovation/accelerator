//! `vcs status`/`vcs log` against the committed golden fixtures, replayed
//! end to end through the compiled `accelerator-vcs` binary over ten real
//! jj/git checkout states, under `vcs-test-support/fixtures/vcs-status-log`.
#![cfg(feature = "bash-parity")]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::masks;

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

fn build_plain_git_states(
    work: &Path,
    env: &Hermetic,
    states: &mut BTreeMap<&'static str, PathBuf>,
) -> Result<(), TestError> {
    let clean_git = work.join("clean-git");
    fs::create_dir_all(&clean_git)?;
    env.git(&["init", "--quiet"], &clean_git)?;
    env.git(
        &["commit", "--allow-empty", "--quiet", "-m", "init"],
        &clean_git,
    )?;
    states.insert("clean-git", clean_git);

    let dirty_git = work.join("dirty-git");
    fs::create_dir_all(&dirty_git)?;
    env.git(&["init", "--quiet"], &dirty_git)?;
    fs::write(dirty_git.join("a.txt"), "A\n")?;
    env.git(&["add", "a.txt"], &dirty_git)?;
    env.git(&["commit", "--quiet", "-m", "init"], &dirty_git)?;
    fs::write(dirty_git.join("a.txt"), "A\nchanged\n")?;
    fs::write(dirty_git.join("untracked.txt"), "untracked\n")?;
    fs::write(dirty_git.join("staged.txt"), "staged\n")?;
    env.git(&["add", "staged.txt"], &dirty_git)?;
    states.insert("dirty-git", dirty_git);

    let detached = work.join("detached-head-git");
    fs::create_dir_all(&detached)?;
    env.git(&["init", "--quiet"], &detached)?;
    fs::write(detached.join("d1.txt"), "1\n")?;
    env.git(&["add", "d1.txt"], &detached)?;
    env.git(&["commit", "--quiet", "-m", "commit-1"], &detached)?;
    let first_sha = env.git(&["rev-parse", "HEAD"], &detached)?;
    fs::write(detached.join("d2.txt"), "2\n")?;
    env.git(&["add", "d2.txt"], &detached)?;
    env.git(&["commit", "--quiet", "-m", "commit-2"], &detached)?;
    env.git(&["checkout", "-q", &first_sha], &detached)?;
    states.insert("detached-head-git", detached);

    Ok(())
}

fn build_ahead_behind_states(
    work: &Path,
    env: &Hermetic,
    states: &mut BTreeMap<&'static str, PathBuf>,
) -> Result<(), TestError> {
    let seed = work.join("seed");
    fs::create_dir_all(&seed)?;
    env.git(&["init", "--quiet"], &seed)?;
    fs::write(seed.join("f.txt"), "1\n")?;
    env.git(&["add", "f.txt"], &seed)?;
    env.git(&["commit", "--quiet", "-m", "commit-1"], &seed)?;
    let origin = work.join("origin.git");
    env.git(
        &[
            "clone",
            "-q",
            "--bare",
            seed.to_str().ok_or("non-utf8 seed path")?,
            origin.to_str().ok_or("non-utf8 origin path")?,
        ],
        work,
    )?;
    let origin = origin.to_str().ok_or("non-utf8 origin path")?;

    let git_ahead = work.join("git-ahead");
    env.git(&["clone", "-q", origin, "git-ahead"], work)?;
    fs::write(git_ahead.join("g.txt"), "2\n")?;
    env.git(&["add", "g.txt"], &git_ahead)?;
    env.git(&["commit", "--quiet", "-m", "commit-2"], &git_ahead)?;
    fs::write(git_ahead.join("h.txt"), "3\n")?;
    env.git(&["add", "h.txt"], &git_ahead)?;
    env.git(&["commit", "--quiet", "-m", "commit-3"], &git_ahead)?;
    states.insert("git-ahead", git_ahead);

    let git_behind = work.join("git-behind");
    env.git(&["clone", "-q", origin, "git-behind"], work)?;
    fs::write(seed.join("i.txt"), "4\n")?;
    env.git(&["add", "i.txt"], &seed)?;
    env.git(&["commit", "--quiet", "-m", "commit-4"], &seed)?;
    env.git(&["push", "-q", origin, "HEAD:refs/heads/main"], &seed)?;
    states.insert("git-behind", git_behind);

    Ok(())
}

fn build_jj_states(
    work: &Path,
    env: &Hermetic,
    states: &mut BTreeMap<&'static str, PathBuf>,
) -> Result<(), TestError> {
    let clean_jj = work.join("clean-jj");
    fs::create_dir_all(&clean_jj)?;
    env.jj(&["git", "init", "--no-colocate"], &clean_jj)?;
    states.insert("clean-jj", clean_jj);

    let dirty_jj = work.join("dirty-jj");
    fs::create_dir_all(&dirty_jj)?;
    env.jj(&["git", "init", "--no-colocate"], &dirty_jj)?;
    fs::write(dirty_jj.join("new-file.txt"), "new content\n")?;
    states.insert("dirty-jj", dirty_jj);

    let colocated = work.join("colocated");
    fs::create_dir_all(&colocated)?;
    env.git(&["init", "--quiet"], &colocated)?;
    env.git(
        &["commit", "--allow-empty", "--quiet", "-m", "init"],
        &colocated,
    )?;
    env.jj(&["git", "init", "--colocate"], &colocated)?;
    states.insert("colocated", colocated);

    let jj_main = work.join("jj-secondary-main");
    fs::create_dir_all(&jj_main)?;
    env.jj(&["git", "init", "--no-colocate"], &jj_main)?;
    let jj_secondary = work.join("jj-secondary");
    env.jj(
        &[
            "workspace",
            "add",
            "--quiet",
            jj_secondary.to_str().ok_or("non-utf8")?,
        ],
        &jj_main,
    )?;
    states.insert("jj-secondary", jj_secondary);

    Ok(())
}

fn build_states(
    work: &Path,
    env: &Hermetic,
) -> Result<BTreeMap<&'static str, PathBuf>, TestError> {
    let mut states = BTreeMap::new();

    build_plain_git_states(work, env, &mut states)?;
    build_ahead_behind_states(work, env, &mut states)?;
    build_jj_states(work, env, &mut states)?;

    let no_repo = work.join("no-repo");
    fs::create_dir_all(&no_repo)?;
    states.insert("no-repo", no_repo);

    Ok(states)
}

#[test]
fn status_and_log_match_every_captured_state() -> Result<(), TestError> {
    let masks = masks::load(&masks_path())?;
    let work = tempfile::Builder::new()
        .prefix("vcs-status-log-goldens-")
        .tempdir()?;
    let env = Hermetic::rooted_at(work.path())?;
    let states = build_states(work.path(), &env)?;

    let mut failures = Vec::new();
    for (name, directory) in &states {
        let rendered_status =
            masks::apply(&masks, &run_vcs("status", directory)?)?;
        let expected_status = golden(&format!("{name}-status"))?;
        if rendered_status.trim_end_matches('\n')
            != expected_status.trim_end_matches('\n')
        {
            failures.push(format!(
                "{name}-status:\n  expected: {expected_status:?}\n  actual:   {rendered_status:?}"
            ));
        }

        let log = masks::apply(&masks, &run_vcs("log", directory)?)?;
        let expected_log = golden(&format!("{name}-log"))?;
        if log.trim_end_matches('\n') != expected_log.trim_end_matches('\n') {
            failures.push(format!(
                "{name}-log:\n  expected: {expected_log:?}\n  actual:   {log:?}"
            ));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
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
    build_plain_git_states(work.path(), &env, &mut states)?;
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
