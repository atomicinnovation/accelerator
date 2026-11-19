//! Each part of the status-log set built privately holds what the same part
//! copied from the full set holds.
//!
//! Under the build step every consumer adopts or copies, so these are the
//! only tests that run a part's private build.

use std::fs;
use std::path::Path;

use vcs_test_support::fixture_log::FixtureLog;
use vcs_test_support::handover::Handover;
use vcs_test_support::status_log::StatusLogPart;
use vcs_test_support::workbench::Workbench;

type TestError = Box<dyn std::error::Error>;

fn private_events(log: &Path) -> Result<usize, TestError> {
    Ok(fs::read_to_string(log)?
        .lines()
        .filter(|line| line.contains("\"event\":\"private-build\""))
        .count())
}

fn assert_the_private_build_matches_the_full_set(
    part: fn() -> StatusLogPart,
) -> Result<(), TestError> {
    let logs = tempfile::tempdir()?;
    let log = logs.path().join("private.jsonl");
    let private = Workbench::with(Handover {
        log: FixtureLog::at(&log),
        ..Handover::nothing()
    })?;
    let built = private.writable_status_log_states(part())?;
    let full = Workbench::for_this_test()?;
    let full = full.status_log_states()?;
    let copied = Workbench::for_this_test()?;
    let copied = copied.writable_status_log_states(part())?;

    let built_keys: Vec<&String> = built.states.keys().collect();
    let mut part_entries = part().entries(&full.base)?;
    part_entries.sort();
    assert_eq!(built_keys, part_entries.iter().collect::<Vec<_>>());
    assert!(built.states.values().all(|state| state.is_dir()));
    assert_eq!(built_keys, copied.states.keys().collect::<Vec<_>>());
    assert!(copied.states.values().all(|state| state.is_dir()));
    assert_eq!(private_events(&log)?, 1);
    Ok(())
}

#[test]
fn the_plain_git_part_built_privately_matches_the_full_set(
) -> Result<(), TestError> {
    assert_the_private_build_matches_the_full_set(|| StatusLogPart::PlainGit)
}

#[test]
fn the_conflict_part_built_privately_matches_the_full_set(
) -> Result<(), TestError> {
    assert_the_private_build_matches_the_full_set(|| StatusLogPart::Conflicts)
}
