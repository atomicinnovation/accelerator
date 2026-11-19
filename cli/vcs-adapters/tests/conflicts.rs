//! A real merge conflict, reported conflicted in process exactly where the
//! `git` and `jj` binaries report one: `git diff --diff-filter=U` and `jj
//! resolve --list` are the oracles.
//!
//! These need real `jj` and `git` binaries and hard-fail when one is absent.

use std::collections::BTreeSet;
use std::path::Path;

use vcs::status::ChangeType;
use vcs::VcsKind;
use vcs::VcsReporter;
use vcs_adapters::library::InProcessProbe;
use vcs_test_support::status_log::StatusLogPart;
use vcs_test_support::workbench::Workbench;

type TestError = Box<dyn std::error::Error>;

fn reported_conflicts(
    root: &Path,
    kind: VcsKind,
) -> Result<BTreeSet<String>, TestError> {
    Ok(InProcessProbe
        .status_report(root, kind)?
        .changes
        .into_iter()
        .filter(|change| change.change_type == ChangeType::Conflicted)
        .map(|change| change.path)
        .collect())
}

#[test]
fn real_merge_conflicts_are_reported_conflicted() -> Result<(), TestError> {
    let workbench = Workbench::for_this_test()?;
    let states =
        workbench.writable_status_log_states(StatusLogPart::Conflicts)?;
    let env = workbench.hermetic();

    let git = states.get("conflict-git")?;
    let git_oracle: BTreeSet<String> = env
        .git(&["diff", "--name-only", "--diff-filter=U"], git)?
        .lines()
        .map(str::to_owned)
        .collect();

    let jj = states.get("conflict-jj")?;
    let jj_oracle: BTreeSet<String> = env
        .jj(&["resolve", "--list"], jj)?
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect();

    assert!(!git_oracle.is_empty(), "the git fixture holds a conflict");
    assert!(!jj_oracle.is_empty(), "the jj fixture holds a conflict");
    assert_eq!(reported_conflicts(git, VcsKind::Git)?, git_oracle);
    assert_eq!(reported_conflicts(jj, VcsKind::Jj)?, jj_oracle);
    Ok(())
}
