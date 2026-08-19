//! `InProcessProbe::is_tracked` against real jj/git repositories — the
//! library-first counterpart of `git ls-files --error-unmatch` / `jj file
//! list`, and the primitive the credential trust boundary rests on.
#![cfg(feature = "bash-parity")]

use std::fs;

use vcs::VcsKind;
use vcs_adapters::library::InProcessProbe;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("vcs-tracked-{tag}-"))
        .tempdir()?)
}

#[test]
fn git_reports_a_committed_file_as_tracked_and_others_as_not(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join(".accelerator"))?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join(".accelerator/config.local.md"), "token: t\n")?;
    env.git(&["add", ".accelerator/config.local.md"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    fs::write(root.join("untracked.md"), "x\n")?;

    let probe = InProcessProbe;
    assert!(
        probe.is_tracked(
            &root,
            ".accelerator/config.local.md",
            VcsKind::Git
        )?,
        "a committed file must read as tracked"
    );
    assert!(
        !probe.is_tracked(&root, "untracked.md", VcsKind::Git)?,
        "a present-but-unadded file must not read as tracked"
    );
    assert!(
        !probe.is_tracked(&root, "does-not-exist.md", VcsKind::Git)?,
        "a nonexistent path must not read as tracked"
    );
    Ok(())
}

#[test]
fn git_reports_a_staged_but_uncommitted_file_as_tracked(
) -> Result<(), TestError> {
    // Matches `git ls-files --error-unmatch`, which reads the index.
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-staged")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    fs::write(root.join("staged.md"), "x\n")?;
    env.git(&["add", "staged.md"], &root)?;

    let probe = InProcessProbe;
    assert!(probe.is_tracked(&root, "staged.md", VcsKind::Git)?);
    Ok(())
}

#[test]
fn jj_reports_a_committed_file_as_tracked_and_an_ignored_one_as_not(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let work = tempdir("jj")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(root.join(".accelerator"))?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join(".gitignore"), "*.log\n")?;
    fs::write(root.join(".accelerator/config.local.md"), "token: t\n")?;
    env.jj(&["commit", "-m", "init"], &root)?;

    fs::write(root.join("secret.log"), "x\n")?;

    let probe = InProcessProbe;
    assert!(
        probe.is_tracked(&root, ".accelerator/config.local.md", VcsKind::Jj)?,
        "a committed file must read as tracked at @"
    );
    assert!(
        !probe.is_tracked(&root, "secret.log", VcsKind::Jj)?,
        "an ignored file is never in the tree, so it must not read as tracked"
    );
    assert!(
        !probe.is_tracked(&root, "does-not-exist.md", VcsKind::Jj)?,
        "a nonexistent path must not read as tracked"
    );
    Ok(())
}

#[test]
fn none_tracks_nothing() -> Result<(), TestError> {
    let work = tempdir("none")?;
    let probe = InProcessProbe;
    assert!(!probe.is_tracked(work.path(), "anything.md", VcsKind::None)?);
    Ok(())
}
