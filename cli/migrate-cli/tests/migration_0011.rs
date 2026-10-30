//! Migration 0011 (`ignore-machine-local-sync-state`) driven end to end over
//! a real git repository, with the ledger pre-seeded through 0010 so 0011 is
//! the sole pending migration.

mod common;

use std::fs;

use common::run;
use common::tempdir;
use common::write;
use common::TestError;
use vcs_test_support::hermetic::Hermetic;

const MARKER: &str =
    ".accelerator/state/integrations/jira/pending-push/0001.json";

const LEDGER_THROUGH_0010: &str = "0001-rename-tickets-to-work\n\
     0002-rename-work-items-with-project-prefix\n\
     0003-relocate-accelerator-state\n\
     0004-restructure-meta-research-into-subject-subcategories\n\
     0005-rename-work-item-type-to-kind\n\
     0006-canonicalise-work-item-id-and-author\n\
     0007-unify-meta-corpus-frontmatter\n\
     0008-canonical-frontmatter-quoting\n\
     0009-split-work-key-from-tracker-scope-key\n\
     0010-strip-research-title-prefix\n";

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
    write(
        &root,
        ".accelerator/state/migrations-applied",
        LEDGER_THROUGH_0010,
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
