//! The shared `vcs status`/`vcs log` fixture states.
//!
//! Built once here so the golden harness (`status_log_goldens.rs`), the parity
//! harness, and the zero-spawn lane each consume one builder rather than
//! duplicating state construction. `build_states` returns the states that carry
//! committed goldens; the cap and bookmark states drive focused assertions and
//! carry no golden, to keep the golden set branch-stable.

use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use tempfile::TempDir;

use crate::hermetic::assert_no_repository_ancestor;
use crate::hermetic::Hermetic;
use crate::Error;

/// Names the root of a status/log state set built by an earlier step, for a
/// caller inside the shadow window that cannot build one itself.
pub const STATES_ROOT_VARIABLE: &str = "ACCELERATOR_ZERO_SPAWN_STATUS_LOG";

/// Written beside the states so a later process can adopt them without git/jj.
const MANIFEST: &str = "status-log-states.tsv";

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

/// The same logical working-copy state built in a git and a jj repo, for the
/// cross-backend parity harness. No golden.
///
/// One modified tracked file and one untracked file over three prior commits,
/// plus one git-only staged change (jj has no staging area). Returned as
/// `parity-git` and `parity-jj`.
///
/// # Errors
///
/// When a fixture cannot be built.
pub fn build_parity_states(
    work: &Path,
    env: &Hermetic,
) -> Result<BTreeMap<&'static str, PathBuf>, Error> {
    let mut states = BTreeMap::new();

    let parity_git = work.join("parity-git");
    fs::create_dir_all(&parity_git)?;
    env.git(&["init", "--quiet"], &parity_git)?;
    fs::write(parity_git.join("tracked.txt"), "v1\n")?;
    env.git(&["add", "tracked.txt"], &parity_git)?;
    env.git(&["commit", "--quiet", "-m", "commit-1"], &parity_git)?;
    for index in 2..=3 {
        fs::write(parity_git.join(format!("c{index}.txt")), "x\n")?;
        env.git(&["add", "."], &parity_git)?;
        env.git(
            &["commit", "--quiet", "-m", &format!("commit-{index}")],
            &parity_git,
        )?;
    }
    fs::write(parity_git.join("tracked.txt"), "v2\n")?;
    fs::write(parity_git.join("untracked.txt"), "new\n")?;
    fs::write(parity_git.join("staged.txt"), "s\n")?;
    env.git(&["add", "staged.txt"], &parity_git)?;
    states.insert("parity-git", parity_git);

    let parity_jj = work.join("parity-jj");
    fs::create_dir_all(&parity_jj)?;
    env.jj(&["git", "init", "--no-colocate"], &parity_jj)?;
    fs::write(parity_jj.join("tracked.txt"), "v1\n")?;
    env.jj(&["commit", "-m", "commit-1"], &parity_jj)?;
    for index in 2..=3 {
        fs::write(parity_jj.join(format!("c{index}.txt")), "x\n")?;
        env.jj(&["commit", "-m", &format!("commit-{index}")], &parity_jj)?;
    }
    fs::write(parity_jj.join("tracked.txt"), "v2\n")?;
    fs::write(parity_jj.join("untracked.txt"), "new\n")?;
    states.insert("parity-jj", parity_jj);

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

/// A git repo whose config is hostile: a filesystem monitor, no golden.
///
/// It declares a `core.fsmonitor`, external content filters bound via
/// `.gitattributes`, a diff textconv, and an external hooks path — every hook a
/// repository could use to induce a spawn. `gix` runs none of them; this state
/// proves a malicious config cannot make the status/log read launch a child.
///
/// # Errors
///
/// When the fixture cannot be built.
pub fn build_adversarial_state(
    work: &Path,
    env: &Hermetic,
    states: &mut BTreeMap<&'static str, PathBuf>,
) -> Result<(), Error> {
    let adversarial = work.join("adversarial-git");
    fs::create_dir_all(&adversarial)?;
    env.git(&["init", "--quiet"], &adversarial)?;
    fs::write(adversarial.join("tracked.dat"), "data\n")?;
    env.git(&["add", "tracked.dat"], &adversarial)?;
    env.git(&["commit", "--quiet", "-m", "init"], &adversarial)?;

    let hooks = adversarial.join("hostile-hooks");
    fs::create_dir_all(&hooks)?;
    let hooks_path = hooks.to_str().ok_or_else(|| non_utf8(&hooks))?;
    for (key, value) in [
        ("core.fsmonitor", "/nonexistent/fsmonitor-hook"),
        ("filter.evil.process", "/nonexistent/evil-process"),
        ("filter.evil.clean", "/nonexistent/evil-clean"),
        ("filter.evil.smudge", "/nonexistent/evil-smudge"),
        ("diff.evil.textconv", "/nonexistent/evil-textconv"),
        ("core.hooksPath", hooks_path),
    ] {
        env.git(&["config", key, value], &adversarial)?;
    }
    fs::write(
        adversarial.join(".gitattributes"),
        "*.dat filter=evil diff=evil\n",
    )?;
    // An untracked *.dat so the status dirwalk would trip the filter if gix ran
    // it, and a worktree edit so the tracked one is re-hashed.
    fs::write(adversarial.join("untracked.dat"), "more\n")?;
    fs::write(adversarial.join("tracked.dat"), "data\nchanged\n")?;
    states.insert("adversarial-git", adversarial);

    Ok(())
}

/// The status/log states beneath a caller-supplied root, adopted inside the
/// zero-spawn shadow window and built (with real git/jj) outside it.
#[derive(Debug)]
pub struct States {
    pub base: PathBuf,
    pub states: BTreeMap<String, PathBuf>,
}

impl States {
    /// Adopts a state set already built beneath `base`, or builds one there.
    ///
    /// Adoption needs neither `git` nor `jj`, so a CI step builds the states
    /// while both are reachable and this consumes them after they are shadowed.
    ///
    /// # Errors
    ///
    /// When the manifest is unreadable or malformed, or a builder fails.
    pub fn build_or_adopt(base: &Path) -> Result<Self, Error> {
        if base.join(MANIFEST).is_file() {
            return Self::adopt(base);
        }
        assert_no_repository_ancestor(base)?;
        let base = base.canonicalize()?;
        let env = Hermetic::rooted_at(&base)?;

        let mut states: BTreeMap<String, PathBuf> = build_states(&base, &env)?
            .into_iter()
            .map(|(key, path)| (key.to_owned(), path))
            .collect();
        let mut extra = BTreeMap::new();
        build_adversarial_state(&base, &env, &mut extra)?;
        states.extend(
            extra.into_iter().map(|(key, path)| (key.to_owned(), path)),
        );

        let built = Self { base, states };
        built.write_manifest()?;
        Ok(built)
    }

    fn write_manifest(&self) -> Result<(), Error> {
        let mut manifest = String::new();
        for (key, path) in &self.states {
            writeln!(manifest, "{key}\t{}", path.display())
                .map_err(|error| Error::message(error.to_string()))?;
        }
        fs::write(self.base.join(MANIFEST), manifest)?;
        Ok(())
    }

    fn adopt(base: &Path) -> Result<Self, Error> {
        let base = base.canonicalize()?;
        let manifest = fs::read_to_string(base.join(MANIFEST))?;
        let mut states = BTreeMap::new();
        for line in manifest.lines() {
            let mut fields = line.split('\t');
            let (Some(key), Some(path), None) =
                (fields.next(), fields.next(), fields.next())
            else {
                return Err(Error::message(format!(
                    "malformed status-log manifest line: {line}"
                )));
            };
            states.insert(key.to_owned(), PathBuf::from(path));
        }
        if states.is_empty() {
            return Err(Error::message(format!(
                "the status-log manifest at {} is empty",
                base.display()
            )));
        }
        Ok(Self { base, states })
    }
}

/// Where the status/log states should live: a caller-supplied root when one is
/// named, otherwise a temp directory the returned guard owns.
///
/// # Errors
///
/// When the temp directory cannot be created.
pub fn states_root() -> Result<(Option<TempDir>, PathBuf), Error> {
    match env::var(STATES_ROOT_VARIABLE) {
        Ok(root) if !root.is_empty() => Ok((None, PathBuf::from(root))),
        _ => {
            let guard = tempfile::Builder::new()
                .prefix("vcs-status-log-")
                .tempdir()?;
            let path = guard.path().to_path_buf();
            Ok((Some(guard), path))
        }
    }
}
