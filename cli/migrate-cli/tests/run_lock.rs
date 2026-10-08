//! The whole-run lock driven end to end against the compiled binary over a
//! committed git repository.
#![cfg(feature = "bash-parity")]

mod common;

use std::fs;

use common::run;
use common::tempdir;
use common::write;
use common::TestError;
use vcs_test_support::hermetic::Hermetic;

const RUN_LOCK: &str = ".accelerator/state/migrate-run.lockdir";

const PENDING_ONLY_0008: &str = "0001-rename-tickets-to-work\n\
     0002-rename-work-items-with-project-prefix\n\
     0003-relocate-accelerator-state\n\
     0004-restructure-meta-research-into-subject-subcategories\n\
     0005-rename-work-item-type-to-kind\n\
     0006-canonicalise-work-item-id-and-author\n\
     0007-unify-meta-corpus-frontmatter\n\
     0009-split-work-key-from-tracker-scope-key\n\
     0010-strip-research-title-prefix\n";

#[test]
fn a_lock_held_by_a_live_run_refuses_a_second_run() -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("live-run-lock")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    write(&root, "README.md", "fixture\n")?;
    write(
        &root,
        ".accelerator/state/migrations-applied",
        PENDING_ONLY_0008,
    )?;
    env.git(&["add", "--all"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;
    let live_pid = std::process::id();
    write(
        &root,
        &format!("{RUN_LOCK}/owner.0123456789abcdef"),
        &live_pid.to_string(),
    )?;

    let outcome = run(&env, &root, &[], &[("ACCELERATOR_LOG", "off")])?;

    assert_eq!(outcome.code, 1);
    assert_eq!(outcome.stdout, "");
    assert_eq!(
        outcome.stderr,
        format!(
            "Another accelerator migrate run is already in progress \
             (pid {live_pid}).\n"
        )
    );
    Ok(())
}
