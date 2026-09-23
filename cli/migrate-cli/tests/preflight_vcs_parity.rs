//! The migrate pre-flight sees exactly the changes `jj status` does: files the
//! git excludes hide are not changes, and everything else still is.
#![cfg(feature = "bash-parity")]

mod common;

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use common::run;
use common::tempdir;
use common::Outcome;
use common::TestError;
use vcs_test_support::hermetic::Hermetic;

const COLOCATIONS: [&str; 2] = ["--no-colocate", "--colocate"];
const IGNORED: &str = "meta/ignored.md";
const VISIBLE: &str = "meta/visible.md";

struct Repo {
    work: tempfile::TempDir,
    env: Hermetic,
    root: PathBuf,
    colocation: &'static str,
}

impl Repo {
    fn jj(colocation: &'static str) -> Result<Self, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let work = tempdir("parity")?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(&root)?;
        env.jj(&["git", "init", colocation], &root)?;
        common::mark_all_migrations_applied(&root)?;
        common::write(&root, "meta/base.md", "base\n")?;
        env.jj(&["commit", "-m", "base"], &root)?;
        Ok(Self {
            work,
            env,
            root,
            colocation,
        })
    }

    fn backing_exclude(&self) -> PathBuf {
        if self.colocation == "--colocate" {
            self.root.join(".git/info/exclude")
        } else {
            self.root.join(".jj/repo/store/git/info/exclude")
        }
    }

    fn with_global_excludes(
        &self,
        patterns: &str,
    ) -> Result<Hermetic, TestError> {
        let excludes = self.work.path().join("global-ignore");
        fs::write(&excludes, patterns)?;
        let config = self.work.path().join("global.gitconfig");
        fs::write(
            &config,
            format!("[core]\n\texcludesFile = {}\n", excludes.display()),
        )?;
        Ok(self.env.clone().with_git_global_config(config))
    }

    fn migrate(&self, env: &Hermetic) -> Result<Outcome, TestError> {
        run(env, &self.root, &[], &[])
    }
}

fn write(path: &Path, content: &str) -> Result<(), TestError> {
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

fn assert_proceeds(outcome: &Outcome) {
    assert_eq!(outcome.code, 0, "{}", outcome.stderr);
    assert!(!outcome.stderr.contains("WARN"), "{}", outcome.stderr);
}

fn assert_refused_listing(outcome: &Outcome, path: &str) {
    assert_eq!(outcome.code, 1, "{}", outcome.stderr);
    assert!(
        outcome
            .stderr
            .ends_with(&format!("Unowned changes (1):\n  {path}\n")),
        "{}",
        outcome.stderr
    );
}

fn an_excluded_file_is_not_a_change(
    hide: impl Fn(&Repo) -> Result<Hermetic, TestError>,
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::jj(colocation)?;
        let env = hide(&repo)?;
        write(&repo.root.join(IGNORED), "ignored\n")?;

        assert_proceeds(&repo.migrate(&env)?);

        write(&repo.root.join(VISIBLE), "visible\n")?;
        assert_refused_listing(&repo.migrate(&env)?, VISIBLE);
    }
    Ok(())
}

#[test]
fn a_file_the_core_excludes_file_ignores_is_not_a_change(
) -> Result<(), TestError> {
    an_excluded_file_is_not_a_change(|repo| {
        repo.with_global_excludes("ignored.md\n")
    })
}

#[test]
fn a_file_info_exclude_ignores_is_not_a_change() -> Result<(), TestError> {
    an_excluded_file_is_not_a_change(|repo| {
        write(&repo.backing_exclude(), "ignored.md\n")?;
        Ok(repo.env.clone())
    })
}

#[test]
fn a_configured_excludes_file_without_the_pattern_leaves_the_file_a_change(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::jj(colocation)?;
        let env = repo.with_global_excludes("unrelated.md\n")?;
        let xdg = repo.env.jj_user_config_dir()?;
        write(&xdg.with_file_name("git").join("ignore"), "ignored.md\n")?;
        write(&repo.root.join(IGNORED), "ignored\n")?;

        assert_refused_listing(&repo.migrate(&env)?, IGNORED);
    }
    Ok(())
}

#[test]
fn a_tracked_file_matching_an_exclude_is_still_a_change(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::jj(colocation)?;
        write(&repo.root.join("meta/tracked.md"), "one\n")?;
        repo.env.jj(&["commit", "-m", "tracked"], &repo.root)?;
        write(&repo.backing_exclude(), "tracked.md\n")?;
        write(&repo.root.join("meta/tracked.md"), "two\n")?;

        assert_refused_listing(&repo.migrate(&repo.env)?, "meta/tracked.md");
    }
    Ok(())
}

fn set_repo_limit(repo: &Repo, value: &str) -> Result<(), TestError> {
    repo.env.jj(
        &[
            "config",
            "set",
            "--repo",
            "snapshot.max-new-file-size",
            value,
        ],
        &repo.root,
    )?;
    Ok(())
}

fn write_sized(
    repo: &Repo,
    relative: &str,
    bytes: usize,
) -> Result<(), TestError> {
    fs::write(repo.root.join(relative), vec![b'x'; bytes])?;
    Ok(())
}

#[test]
fn a_new_file_over_the_limit_is_not_a_change() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::jj(colocation)?;
        set_repo_limit(&repo, "1KiB")?;
        write_sized(&repo, "meta/two", 2048)?;
        write_sized(&repo, "meta/one", 1024)?;
        write_sized(&repo, "meta/half", 512)?;

        let refused = repo.migrate(&repo.env)?;

        assert_eq!(refused.code, 1, "{}", refused.stderr);
        assert!(
            refused
                .stderr
                .ends_with("Unowned changes (2):\n  meta/half\n  meta/one\n"),
            "{colocation}: {}",
            refused.stderr
        );
    }
    Ok(())
}

#[test]
fn a_new_file_over_the_default_limit_is_not_a_change() -> Result<(), TestError>
{
    for colocation in COLOCATIONS {
        let repo = Repo::jj(colocation)?;
        write_sized(&repo, "meta/large", 2 * 1024 * 1024)?;

        assert_proceeds(&repo.migrate(&repo.env)?);

        write_sized(&repo, "meta/half", 512)?;
        assert_refused_listing(&repo.migrate(&repo.env)?, "meta/half");
    }
    Ok(())
}

#[test]
fn an_invalid_limit_is_warned_about_and_every_file_is_a_change(
) -> Result<(), TestError> {
    let repo = Repo::jj("--no-colocate")?;
    set_repo_limit(&repo, "\"abc\"")?;
    write(&repo.root.join("meta/a.md"), "a\n")?;
    write_sized(&repo, "meta/two", 2 * 1024 * 1024)?;

    let refused = repo.migrate(&repo.env)?;

    assert_eq!(refused.code, 1, "{}", refused.stderr);
    assert!(refused.stderr.contains("WARN"), "{}", refused.stderr);
    assert!(
        refused.stderr.contains("snapshot.max-new-file-size"),
        "{}",
        refused.stderr
    );
    assert!(
        refused
            .stderr
            .ends_with("Unowned changes (2):\n  meta/a.md\n  meta/two\n"),
        "{}",
        refused.stderr
    );
    Ok(())
}
