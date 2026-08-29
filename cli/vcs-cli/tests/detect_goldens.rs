//! `vcs detect` against the committed golden fixtures, replayed end to end
//! through the compiled `accelerator-vcs` binary over real jj/git checkouts.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use vcs_test_support::fixtures::pure_jj;
use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-vcs");

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("vcs-detect-golden-{tag}-"))
        .tempdir()?)
}

fn fixture(name: &str) -> Result<Value, TestError> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../vcs-test-support/fixtures/vcs-detect")
        .join(name);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}

fn rendered(
    start: &Path,
    descriptive: bool,
) -> Result<Option<Value>, TestError> {
    let mut command = Command::new(BIN);
    command.current_dir(start);
    command.arg("detect");
    command.arg("--format=hook");
    if descriptive {
        command.arg("--descriptive");
    }
    let output = command.output()?;
    assert!(
        output.status.success(),
        "accelerator-vcs detect exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    if stdout.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(stdout.trim())?))
}

#[test]
fn main_jj_workspace_matches_its_descriptive_golden() -> Result<(), TestError> {
    let work = tempdir("main-jj")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("main-jj");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    env.jj(&["git", "init", "--colocate"], &root)?;

    let output = rendered(&root, true)?.ok_or("expected output")?;
    assert_eq!(output, fixture("main-jj-workspace.json")?);
    Ok(())
}

#[test]
fn main_git_checkout_matches_its_descriptive_golden() -> Result<(), TestError> {
    let work = tempdir("main-git")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("main-git");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "init"], &root)?;

    let output = rendered(&root, true)?.ok_or("expected output")?;
    assert_eq!(output, fixture("main-git-checkout.json")?);
    Ok(())
}

#[test]
fn a_colocated_checkout_with_git_as_a_file_matches_its_descriptive_golden(
) -> Result<(), TestError> {
    let work = tempdir("colocated-file")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("colocated");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    env.jj(&["git", "init", "--colocate"], &root)?;

    // Move the real `.git` directory aside and replace it with a `gitdir:`
    // redirect file — the shape a linked worktree or submodule leaves behind,
    // which `[ -d "$ROOT/.git" ]` misreads as absent but `gix::discover`
    // (behind `dual_roots().git`) follows correctly.
    let real_git = root.join(".git-real");
    fs::rename(root.join(".git"), &real_git)?;
    fs::write(
        root.join(".git"),
        format!("gitdir: {}\n", real_git.display()),
    )?;

    let output = rendered(&root, true)?.ok_or("expected output")?;
    assert_eq!(output, fixture("colocated-git-as-file.json")?);
    Ok(())
}

fn mask_paths(mut value: Value, boundary: &Path, jj_parent: &Path) -> Value {
    let Value::Object(map) = &mut value else {
        return value;
    };
    let Some(Value::Object(output)) = map.get_mut("hookSpecificOutput") else {
        return value;
    };
    let Some(Value::String(context)) = output.get_mut("additionalContext")
    else {
        return value;
    };
    *context = context
        .replace(&boundary.display().to_string(), "<BOUNDARY_PATH>")
        .replace(&jj_parent.display().to_string(), "<JJ_PARENT_PATH>");
    value
}

#[test]
fn a_jj_secondary_workspace_without_descriptive_matches_its_structured_golden(
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
    let main = main.canonicalize()?;

    let output = rendered(&secondary, false)?.ok_or("expected output")?;
    assert_eq!(
        mask_paths(output, &secondary, &main),
        fixture("jj-secondary-structured.json")?
    );
    Ok(())
}

#[test]
fn a_main_checkout_without_descriptive_produces_zero_bytes(
) -> Result<(), TestError> {
    let work = tempdir("nothing-to-report")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("main-git");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    env.git(&["commit", "--allow-empty", "--quiet", "-m", "init"], &root)?;

    let output = rendered(&root, false)?;
    assert_eq!(output, None);
    Ok(())
}
