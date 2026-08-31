//! The status/log zero-spawn states build, and every state carries a start
//! directory.
//!
//! Runs with the real `git`/`jj` so the CI job can build the states while both
//! are reachable and hand the root over for the shadowed suite to adopt.

use vcs_test_support::status_log;

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
