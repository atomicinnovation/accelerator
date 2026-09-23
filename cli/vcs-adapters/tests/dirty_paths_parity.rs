//! The jj dirty paths, read through the fixture binary under a hermetic
//! environment, against the paths on `jj status`'s change lines.
#![cfg(feature = "bash-parity")]

mod support;

use std::fs;
use std::path::PathBuf;

use support::dirty_paths;
use support::TestError;
use tempfile::TempDir;
use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::jj_status::changed_paths;

const COLOCATIONS: [&str; 2] = ["--no-colocate", "--colocate"];

struct JjRepo {
    _work: TempDir,
    env: Hermetic,
    root: PathBuf,
}

impl JjRepo {
    fn new(colocation: &str) -> Result<Self, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let work = tempfile::Builder::new()
            .prefix("vcs-dirty-parity-")
            .tempdir()?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(&root)?;
        env.jj(&["git", "init", colocation], &root)?;
        Ok(Self {
            _work: work,
            env,
            root,
        })
    }

    fn write(&self, relative: &str, content: &str) -> Result<(), TestError> {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn rename(&self, from: &str, to: &str) -> Result<(), TestError> {
        let target = self.root.join(to);
        fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        fs::rename(self.root.join(from), target)?;
        Ok(())
    }

    fn jj(&self, args: &[&str]) -> Result<String, TestError> {
        Ok(self.env.jj(args, &self.root)?)
    }

    fn assert_parity(&self) -> Result<(), TestError> {
        let reported = dirty_paths(&self.env, &self.root)?;
        let oracle = changed_paths(&self.env, &self.root)?;
        assert_eq!(reported, oracle);
        Ok(())
    }
}

#[test]
fn additions_edits_and_deletions_match_jj_status() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = JjRepo::new(colocation)?;
        repo.write("meta/kept.md", "kept")?;
        repo.write("meta/gone.md", "gone")?;
        repo.jj(&["commit", "-m", "init"])?;
        repo.write("meta/kept.md", "edited")?;
        fs::remove_file(repo.root.join("meta/gone.md"))?;
        repo.write("meta/new.md", "new")?;
        repo.write(".accelerator/state/migrations-run.id", "abc\n")?;

        repo.assert_parity()?;
    }
    Ok(())
}

#[test]
fn renames_report_both_sides_as_jj_status_does() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = JjRepo::new(colocation)?;
        repo.write("meta/a.md", "a")?;
        repo.write("meta/x/c.md", "c")?;
        repo.write("top.md", "t")?;
        repo.jj(&["commit", "-m", "init"])?;
        repo.rename("meta/a.md", "meta/b.md")?;
        repo.rename("meta/x/c.md", "meta/y/c.md")?;
        repo.rename("top.md", "moved.md")?;

        repo.assert_parity()?;
    }
    Ok(())
}

#[test]
fn a_merge_working_copy_matches_jj_status() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = JjRepo::new(colocation)?;
        repo.write("meta/a.md", "a")?;
        repo.jj(&["commit", "-m", "a"])?;
        let a =
            repo.jj(&["log", "--no-graph", "-r", "@-", "-T", "change_id"])?;
        repo.jj(&["new", "root()"])?;
        repo.write("meta/b.md", "b")?;
        repo.jj(&["commit", "-m", "b"])?;
        let b =
            repo.jj(&["log", "--no-graph", "-r", "@-", "-T", "change_id"])?;
        repo.jj(&["new", &a, &b])?;
        repo.write("meta/a.md", "edited on the merge")?;
        repo.write("meta/merged.md", "new on the merge")?;

        repo.assert_parity()?;
    }
    Ok(())
}

#[test]
fn a_sibling_change_matches_jj_status() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = JjRepo::new(colocation)?;
        repo.write("meta/a.md", "a")?;
        repo.jj(&["commit", "-m", "base"])?;
        repo.write("meta/sibling.md", "sibling")?;
        let sibling =
            repo.jj(&["log", "--no-graph", "-r", "@", "-T", "change_id"])?;
        repo.jj(&["new", "@-"])?;
        repo.jj(&["edit", &sibling])?;

        repo.assert_parity()?;
    }
    Ok(())
}
