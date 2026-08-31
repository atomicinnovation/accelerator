//! The shared `vcs status`/`vcs log` fixture states.
//!
//! Built once here so the golden harness (`status_log_goldens.rs`), the parity
//! harness, and the zero-spawn lane each consume one builder — Phase 3 does not
//! transitively depend on Phase 2. `build_states` returns the states that carry
//! committed goldens; the cap and bookmark states drive focused assertions and
//! carry no golden, to keep the golden set branch-stable.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::hermetic::Hermetic;
use crate::Error;

fn non_utf8(path: &Path) -> Error {
    Error::message(format!("non-utf8 path: {}", path.display()))
}

/// Clean, dirty, and detached git checkouts.
///
/// `pub` because the fail-safe and malformed-log tests reach only this family.
///
/// # Errors
///
/// When a fixture cannot be built.
pub fn build_plain_git_states(
    work: &Path,
    env: &Hermetic,
    states: &mut BTreeMap<&'static str, PathBuf>,
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
            seed.to_str().ok_or_else(|| non_utf8(&seed))?,
            origin.to_str().ok_or_else(|| non_utf8(&origin))?,
        ],
        work,
    )?;
    let origin = origin.to_str().ok_or_else(|| non_utf8(&origin))?;

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
) -> Result<(), Error> {
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
            jj_secondary
                .to_str()
                .ok_or_else(|| non_utf8(&jj_secondary))?,
        ],
        &jj_main,
    )?;
    states.insert("jj-secondary", jj_secondary);

    Ok(())
}

fn build_conflict_states(
    work: &Path,
    env: &Hermetic,
    states: &mut BTreeMap<&'static str, PathBuf>,
) -> Result<(), Error> {
    let conflict_git = work.join("conflict-git");
    fs::create_dir_all(&conflict_git)?;
    env.git(&["init", "--quiet"], &conflict_git)?;
    fs::write(conflict_git.join("f.txt"), "base\n")?;
    env.git(&["add", "f.txt"], &conflict_git)?;
    env.git(&["commit", "--quiet", "-m", "base"], &conflict_git)?;
    env.git(&["checkout", "-q", "-b", "feature"], &conflict_git)?;
    fs::write(conflict_git.join("f.txt"), "feature\n")?;
    env.git(&["commit", "--quiet", "-a", "-m", "feature"], &conflict_git)?;
    env.git(&["checkout", "-q", "main"], &conflict_git)?;
    fs::write(conflict_git.join("f.txt"), "mainline\n")?;
    env.git(
        &["commit", "--quiet", "-a", "-m", "mainline"],
        &conflict_git,
    )?;
    // Leaves f.txt conflicted; the merge exits non-zero by design.
    env.git_allow_failure(&["merge", "feature"], &conflict_git)?;
    states.insert("conflict-git", conflict_git);

    let conflict_jj = work.join("conflict-jj");
    fs::create_dir_all(&conflict_jj)?;
    env.jj(&["git", "init", "--no-colocate"], &conflict_jj)?;
    fs::write(conflict_jj.join("f.txt"), "base\n")?;
    env.jj(&["commit", "-m", "base"], &conflict_jj)?;
    env.jj(&["new", "@-", "-m", "left"], &conflict_jj)?;
    fs::write(conflict_jj.join("f.txt"), "left\n")?;
    env.jj(&["bookmark", "create", "bm-left", "-r", "@"], &conflict_jj)?;
    env.jj(&["new", "@-", "-m", "right"], &conflict_jj)?;
    fs::write(conflict_jj.join("f.txt"), "right\n")?;
    env.jj(&["bookmark", "create", "bm-right", "-r", "@"], &conflict_jj)?;
    env.jj(&["new", "bm-left", "bm-right"], &conflict_jj)?;
    states.insert("conflict-jj", conflict_jj);

    Ok(())
}

fn build_change_type_states(
    work: &Path,
    env: &Hermetic,
    states: &mut BTreeMap<&'static str, PathBuf>,
) -> Result<(), Error> {
    let deleted_git = work.join("deleted-git");
    fs::create_dir_all(&deleted_git)?;
    env.git(&["init", "--quiet"], &deleted_git)?;
    fs::write(deleted_git.join("gone.txt"), "gone\n")?;
    env.git(&["add", "gone.txt"], &deleted_git)?;
    env.git(&["commit", "--quiet", "-m", "add gone"], &deleted_git)?;
    fs::remove_file(deleted_git.join("gone.txt"))?;
    states.insert("deleted-git", deleted_git);

    let deleted_jj = work.join("deleted-jj");
    fs::create_dir_all(&deleted_jj)?;
    env.jj(&["git", "init", "--no-colocate"], &deleted_jj)?;
    fs::write(deleted_jj.join("gone.txt"), "gone\n")?;
    env.jj(&["commit", "-m", "add gone"], &deleted_jj)?;
    fs::remove_file(deleted_jj.join("gone.txt"))?;
    states.insert("deleted-jj", deleted_jj);

    let rename_git = work.join("rename-git");
    fs::create_dir_all(&rename_git)?;
    env.git(&["init", "--quiet"], &rename_git)?;
    fs::write(rename_git.join("old.txt"), "content\n")?;
    env.git(&["add", "old.txt"], &rename_git)?;
    env.git(&["commit", "--quiet", "-m", "add old"], &rename_git)?;
    env.git(&["mv", "old.txt", "new.txt"], &rename_git)?;
    states.insert("rename-git", rename_git);

    let rename_jj = work.join("rename-jj");
    fs::create_dir_all(&rename_jj)?;
    env.jj(&["git", "init", "--no-colocate"], &rename_jj)?;
    fs::write(rename_jj.join("old.txt"), "content\n")?;
    env.jj(&["commit", "-m", "add old"], &rename_jj)?;
    fs::remove_file(rename_jj.join("old.txt"))?;
    fs::write(rename_jj.join("new.txt"), "content\n")?;
    states.insert("rename-jj", rename_jj);

    Ok(())
}

fn build_empty_history_states(
    work: &Path,
    env: &Hermetic,
    states: &mut BTreeMap<&'static str, PathBuf>,
) -> Result<(), Error> {
    let unborn_git = work.join("unborn-git");
    fs::create_dir_all(&unborn_git)?;
    env.git(&["init", "--quiet"], &unborn_git)?;
    states.insert("unborn-git", unborn_git);

    let empty_jj = work.join("empty-jj");
    fs::create_dir_all(&empty_jj)?;
    env.jj(&["git", "init", "--no-colocate"], &empty_jj)?;
    states.insert("empty-jj", empty_jj);

    Ok(())
}

fn build_sha256_git_state(
    work: &Path,
    env: &Hermetic,
    states: &mut BTreeMap<&'static str, PathBuf>,
) -> Result<(), Error> {
    let sha256_git = work.join("sha256-git");
    fs::create_dir_all(&sha256_git)?;
    env.git(&["init", "--quiet", "--object-format=sha256"], &sha256_git)?;
    fs::write(sha256_git.join("a.txt"), "A\n")?;
    env.git(&["add", "a.txt"], &sha256_git)?;
    env.git(&["commit", "--quiet", "-m", "init"], &sha256_git)?;
    states.insert("sha256-git", sha256_git);

    Ok(())
}

/// Every state that carries a committed golden.
///
/// # Errors
///
/// When a fixture cannot be built (git/jj missing, or a filesystem failure).
pub fn build_states(
    work: &Path,
    env: &Hermetic,
) -> Result<BTreeMap<&'static str, PathBuf>, Error> {
    let mut states = BTreeMap::new();

    build_plain_git_states(work, env, &mut states)?;
    build_ahead_behind_states(work, env, &mut states)?;
    build_jj_states(work, env, &mut states)?;
    build_conflict_states(work, env, &mut states)?;
    build_change_type_states(work, env, &mut states)?;
    build_empty_history_states(work, env, &mut states)?;
    build_sha256_git_state(work, env, &mut states)?;

    let no_repo = work.join("no-repo");
    fs::create_dir_all(&no_repo)?;
    states.insert("no-repo", no_repo);

    Ok(states)
}

/// The states for the five-commit-cap assertion, carrying no golden.
///
/// A git repo of seven commits (`HEAD` plus six ancestors) and a jj repo of six
/// commits below the empty working-copy commit — the cap is asserted
/// structurally, not by golden.
///
/// # Errors
///
/// When a fixture cannot be built.
pub fn build_cap_states(
    work: &Path,
    env: &Hermetic,
) -> Result<BTreeMap<&'static str, PathBuf>, Error> {
    let mut states = BTreeMap::new();

    let cap_git = work.join("cap-git");
    fs::create_dir_all(&cap_git)?;
    env.git(&["init", "--quiet"], &cap_git)?;
    for index in 1..=7 {
        fs::write(cap_git.join(format!("f{index}.txt")), format!("{index}\n"))?;
        env.git(&["add", "."], &cap_git)?;
        env.git(
            &["commit", "--quiet", "-m", &format!("commit-{index}")],
            &cap_git,
        )?;
    }
    states.insert("cap-git", cap_git);

    let cap_jj = work.join("cap-jj");
    fs::create_dir_all(&cap_jj)?;
    env.jj(&["git", "init", "--no-colocate"], &cap_jj)?;
    for index in 1..=6 {
        fs::write(cap_jj.join(format!("f{index}.txt")), format!("{index}\n"))?;
        env.jj(&["commit", "-m", &format!("commit-{index}")], &cap_jj)?;
    }
    states.insert("cap-jj", cap_jj);

    Ok(states)
}

/// The states for the bookmark-header assertion, carrying no golden.
///
/// A jj working-copy commit carrying one bookmark, and one carrying two —
/// asserted structurally to keep the golden set branch-stable.
///
/// # Errors
///
/// When a fixture cannot be built.
pub fn build_bookmark_states(
    work: &Path,
    env: &Hermetic,
) -> Result<BTreeMap<&'static str, PathBuf>, Error> {
    let mut states = BTreeMap::new();

    let one = work.join("bookmark-one-jj");
    fs::create_dir_all(&one)?;
    env.jj(&["git", "init", "--no-colocate"], &one)?;
    env.jj(&["bookmark", "create", "solo", "-r", "@"], &one)?;
    states.insert("bookmark-one-jj", one);

    let two = work.join("bookmark-two-jj");
    fs::create_dir_all(&two)?;
    env.jj(&["git", "init", "--no-colocate"], &two)?;
    env.jj(&["bookmark", "create", "zed", "-r", "@"], &two)?;
    env.jj(&["bookmark", "create", "alpha", "-r", "@"], &two)?;
    states.insert("bookmark-two-jj", two);

    Ok(states)
}
