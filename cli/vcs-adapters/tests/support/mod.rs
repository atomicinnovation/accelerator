//! Drives `vcs-adapters-fixture` under a hermetic environment, for tests
//! whose outcome depends on the environment the adapter reads.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::process::Output;

use vcs_test_support::hermetic::Hermetic;

pub type TestError = Box<dyn std::error::Error>;

const FIXTURE: &str = env!("CARGO_BIN_EXE_vcs-adapters-fixture");

/// Runs one `only <query>` of the fixture with warnings logged.
pub fn query(
    env: &Hermetic,
    name: &str,
    root: &Path,
) -> Result<Output, TestError> {
    let mut command = Command::new(FIXTURE);
    command.arg("only").arg(name).arg(root);
    env.apply(&mut command);
    command.env("ACCELERATOR_LOG", "warn");
    Ok(command.output()?)
}

fn succeeded(output: Output) -> Result<String, TestError> {
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .into_owned()
            .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

pub fn dirty_paths(
    env: &Hermetic,
    root: &Path,
) -> Result<BTreeSet<String>, TestError> {
    Ok(succeeded(query(env, "dirty_paths", root)?)?
        .lines()
        .map(str::to_owned)
        .collect())
}

pub struct WorkingCopyState {
    pub base_commits: Vec<String>,
    pub dirty_paths: BTreeSet<String>,
}

pub fn working_copy_state(
    env: &Hermetic,
    root: &Path,
) -> Result<WorkingCopyState, TestError> {
    let listing = succeeded(query(env, "working_copy_state", root)?)?;
    let mut base_commits = Vec::new();
    let mut dirty_paths = BTreeSet::new();
    for line in listing.lines() {
        match line.split_once('\t') {
            Some(("base", commit)) => base_commits.push(commit.to_owned()),
            Some(("dirty", path)) => {
                dirty_paths.insert(path.to_owned());
            }
            _ => return Err(format!("unexpected fixture line {line:?}").into()),
        }
    }
    base_commits.sort_unstable();
    Ok(WorkingCopyState {
        base_commits,
        dirty_paths,
    })
}

pub fn revision(env: &Hermetic, root: &Path) -> Result<String, TestError> {
    let listing = succeeded(query(env, "kind_and_revision", root)?)?;
    let rendered = listing
        .strip_prefix("kind_and_revision\t")
        .ok_or("no kind_and_revision line")?;
    Ok(rendered
        .split_once(' ')
        .ok_or("no revision")?
        .1
        .trim()
        .to_owned())
}
