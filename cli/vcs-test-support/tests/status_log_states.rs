//! The status-log set builds or is adopted whole, and no state records the
//! directory it was built in, so a copy of it reads the same.

use std::path::PathBuf;

use vcs_test_support::path_scan::files_naming;
use vcs_test_support::workbench::Workbench;

type TestError = Box<dyn std::error::Error>;

#[test]
fn every_status_log_state_is_built() -> Result<(), TestError> {
    let workbench = Workbench::for_this_test()?;
    let states = workbench.status_log_states()?;

    assert!(
        !states.states.is_empty(),
        "the status/log state set must not be empty"
    );
    for expected in ["clean-git", "dirty-jj", "conflict-git", "adversarial-git"]
    {
        assert!(
            states.states.contains_key(expected),
            "the state set must carry {expected}"
        );
    }
    Ok(())
}

#[test]
fn no_state_names_the_directory_it_was_built_in() -> Result<(), TestError> {
    let workbench = Workbench::for_this_test()?;
    let states = workbench.status_log_states()?;

    let supporting_repositories = ["seed", "origin.git", "jj-secondary-main"]
        .map(|name| states.base.join(name));
    let mut naming = Vec::<PathBuf>::new();
    for tree in states.states.values().chain(&supporting_repositories) {
        naming.extend(files_naming(tree, &states.base)?);
    }

    assert!(
        naming.is_empty(),
        "these files name the directory the states were built in, so a copy \
         would still point at it:\n{}",
        naming
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}
