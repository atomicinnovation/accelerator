//! `vcs root` against real git/jj checkouts: the printed working-copy root
//! equals the checkout's own canonical root under every topology, and in a jj
//! secondary workspace it is the workspace root, not the shared main
//! repository — the discriminating case for surfacing `discover` rather than
//! `repository_root`.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::process::Command;

use vcs_test_support::fixtures::pure_jj;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-vcs");

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("vcs-root-golden-{tag}-"))
        .tempdir()?)
}

fn root_of(start: &Path) -> Result<String, TestError> {
    let output = Command::new(BIN).current_dir(start).arg("root").output()?;
    assert!(
        output.status.success(),
        "accelerator-vcs root exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[test]
fn a_git_only_checkout_prints_its_own_root() -> Result<(), TestError> {
    let work = tempdir("git-only")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;

    assert_eq!(root_of(&root)?, root.canonicalize()?.display().to_string());
    Ok(())
}

#[test]
fn a_colocated_checkout_prints_its_own_root() -> Result<(), TestError> {
    let work = tempdir("colocated")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    env.jj(&["git", "init", "--colocate"], &root)?;

    assert_eq!(root_of(&root)?, root.canonicalize()?.display().to_string());
    Ok(())
}

#[test]
fn a_pure_jj_checkout_prints_its_own_root() -> Result<(), TestError> {
    let work = tempdir("pure-jj")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = pure_jj(&work.path().join("repo"), &env)
        .map_err(|error| error.to_string())?;

    assert_eq!(root_of(&root)?, root.display().to_string());
    Ok(())
}

#[test]
fn a_jj_secondary_workspace_prints_the_workspace_root_not_the_main_repo(
) -> Result<(), TestError> {
    let work = tempdir("jj-secondary")?;
    let env = Hermetic::rooted_at(work.path())?;
    let main = pure_jj(&work.path().join("main"), &env)
        .map_err(|error| error.to_string())?;
    let secondary = work.path().join("secondary");
    env.jj(
        &["workspace", "add", secondary.to_str().ok_or("non-utf8")?],
        &main,
    )?;
    let secondary = secondary.canonicalize()?;

    let printed = root_of(&secondary)?;
    assert_eq!(printed, secondary.display().to_string());
    assert_ne!(printed, main.display().to_string());
    Ok(())
}
