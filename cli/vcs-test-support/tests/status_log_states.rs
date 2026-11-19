//! The status/log zero-spawn states build, and every state carries a start
//! directory.
//!
//! Runs with the real `git`/`jj` so the CI job can build the states while both
//! are reachable and hand the root over for the shadowed suite to adopt.

use std::path::PathBuf;

use vcs_test_support::path_scan::files_naming;
use vcs_test_support::status_log;
use vcs_test_support::status_log::States;

type TestError = Box<dyn std::error::Error>;

#[test]
fn every_status_log_state_is_built() -> Result<(), TestError> {
    let (_guard, root) = status_log::states_root()?;
    let states = status_log::States::build_or_adopt(&root)?;

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
    let directory = tempfile::tempdir()?;
    let states = States::build_in(directory.path())?;

    let supporting_repositories = ["seed", "origin.git", "jj-secondary-main"]
        .map(|name| states.base.join(name));
    let mut naming = Vec::<PathBuf>::new();
    for tree in states.states.values().chain(&supporting_repositories) {
        naming.extend(files_naming(tree, directory.path())?);
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
