//! Migration 0011 (`ignore-machine-local-sync-state`) driven end to end over
//! a real git repository, with every other migration pre-seeded in the
//! ledger, so 0011 is the sole pending migration.

mod common;

use std::fs;

use common::mark_all_migrations_applied_except;
use common::run;
use common::tempdir;
use common::write;
use common::TestError;
use vcs_test_support::hermetic::Hermetic;

const MARKER: &str =
    ".accelerator/state/integrations/jira/pending-push/0001.json";

#[test]
fn a_tracked_marker_is_named_with_its_untrack_command_and_left_tracked(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("0011-tracked-marker")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    write(&root, ".accelerator/.gitignore", "config.local.md\n")?;
    mark_all_migrations_applied_except(
        &root,
        "0011-ignore-machine-local-sync-state",
    )?;
    write(&root, MARKER, "{\"state\":\"attempted\"}\n")?;
    env.git(&["add", "--all"], &root)?;
    env.git(&["commit", "--quiet", "-m", "a committed marker"], &root)?;

    let outcome = run(&env, &root, &[], &[])?;

    assert_eq!(outcome.code, 0, "{}", outcome.stderr);
    assert!(
        outcome
            .stderr
            .contains(&format!("\n  {MARKER}  (git rm --cached -r {MARKER})")),
        "{}",
        outcome.stderr
    );
    assert!(fs::read_to_string(root.join(".accelerator/.gitignore"))?
        .lines()
        .any(|line| line == "state/integrations/*/pending-push/"));
    let tracked = env.git(&["ls-files", MARKER], &root)?;
    assert!(tracked.contains(MARKER), "nothing is untracked: {tracked}");
    Ok(())
}
