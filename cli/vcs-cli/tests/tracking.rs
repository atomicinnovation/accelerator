//! `vcs tracking --path` against real git repositories. Every answer is read
//! back through `kernel::TrackingAnswer`, the type the launcher parses it
//! with, so a renamed token fails here rather than silently at session start.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use kernel::TrackingAnswer;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-vcs");
const PERSONAL: &str = ".accelerator/config.local.md";

struct Repository {
    _dir: tempfile::TempDir,
    root: PathBuf,
    env: Hermetic,
}

impl Repository {
    fn git() -> Result<Self, TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let dir = tempfile::Builder::new().prefix("vcs-tracking-").tempdir()?;
        let base = dir.path().canonicalize()?;
        let env = Hermetic::rooted_at(&base)?;
        let root = base.join("repo");
        fs::create_dir_all(root.join(".accelerator"))?;
        env.git(&["init", "--quiet"], &root)?;
        fs::write(root.join(PERSONAL), "---\njira:\n  token_cmd: x\n---\n")?;
        Ok(Self {
            _dir: dir,
            root,
            env,
        })
    }

    fn personal(&self) -> PathBuf {
        self.root.join(PERSONAL)
    }

    fn stage_personal(&self) -> Result<(), TestError> {
        self.env.git(&["add", PERSONAL], &self.root)?;
        Ok(())
    }
}

fn tracking(path: &Path) -> Result<Output, TestError> {
    Ok(Command::new(BIN)
        .arg("tracking")
        .arg("--path")
        .arg(path)
        .output()?)
}

fn answer(output: &Output) -> Result<TrackingAnswer, TestError> {
    assert!(
        output.status.success(),
        "accelerator-vcs tracking exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout.clone())?.parse()?)
}

#[test]
fn an_untracked_file_is_untracked() -> Result<(), TestError> {
    let repository = Repository::git()?;

    let output = tracking(&repository.personal())?;

    assert_eq!(answer(&output)?, TrackingAnswer::Untracked);
    Ok(())
}

#[test]
fn a_staged_file_is_tracked() -> Result<(), TestError> {
    let repository = Repository::git()?;
    repository.stage_personal()?;

    let output = tracking(&repository.personal())?;

    assert_eq!(answer(&output)?, TrackingAnswer::Tracked);
    Ok(())
}

#[test]
fn a_corrupted_index_exits_non_zero_with_no_answer() -> Result<(), TestError> {
    let repository = Repository::git()?;
    repository.stage_personal()?;
    fs::write(
        repository.root.join(".git/index"),
        b"DIRC not really an index",
    )?;

    let output = tracking(&repository.personal())?;

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    Ok(())
}
