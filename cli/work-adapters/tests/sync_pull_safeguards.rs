//! A pull over a real file: the edits it keeps before writing, decided by
//! the run from the item's baseline entry, and the edits it refuses to
//! overwrite.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use std::path::PathBuf;

use work::sync::Action;
use work::sync::Resolution;
use work::sync::SyncState;
use work_adapters::sync::apply::ApplyError;
use work_adapters::sync::apply::FailureClass;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::RunReport;

use crate::support::repo::issue;
use crate::support::repo::outcome_of;
use crate::support::repo::row;
use crate::support::repo::work_item;
use crate::support::repo::Recorded;
use crate::support::repo::Repo;
use crate::support::repo::Tracker;
use crate::support::repo::MOVED_STAMP;

const TITLE: &str = "Add search";
const SYNCED_BODY: &str = "Body as last synced\n";
const EDITED_BODY: &str = "Body edited and committed since\n";
const KEPT_DIR: &str = "sync-recovery/1700000000-000000-0001";

fn content(body: &str) -> String {
    work_item(1, TITLE, "status: ready\n", body)
}

fn projection(body: &str) -> String {
    format!("{TITLE}\n{body}")
}

fn remote_moved() -> tracker::RemoteIssue {
    issue(1, &projection("Body the remote moved to\n"), MOVED_STAMP)
}

fn kept_edits(report: &RunReport) -> Option<PathBuf> {
    row(report, 1).kept_edits.clone()
}

/// Item 1 edited locally since it synced `recorded`, while its remote
/// moved too, so it conflicts and its pull comes from a resolution.
fn conflicting(recorded: &Recorded<'_>) -> Repo {
    Repo::with_item(
        Tracker::Linear,
        1,
        &content(EDITED_BODY),
        remote_moved(),
        recorded,
    )
}

fn assert_pulled_keeping_a_copy(repo: &Repo, report: &RunReport) {
    assert_eq!(
        outcome_of(report, 1),
        (SyncState::Conflict, Action::Pull),
        "the resolution pulls"
    );
    assert!(
        matches!(row(report, 1).outcome, ItemOutcome::Applied),
        "the pull lands"
    );
    assert_eq!(kept_edits(report), Some(PathBuf::from(KEPT_DIR)));
    assert_eq!(repo.kept_edits(), vec![PathBuf::from(KEPT_DIR)]);
}

#[test]
fn a_committed_edit_since_the_last_sync_keeps_a_copy_on_a_resolution_pull() {
    let synced = content(SYNCED_BODY);
    let recorded = projection(SYNCED_BODY);
    let repo = conflicting(&Recorded::split(&synced, &recorded));

    let report = repo.resolving(1, Resolution::AcceptRemote);

    assert_pulled_keeping_a_copy(&repo, &report);
}

#[test]
fn a_committed_edit_under_a_legacy_entry_keeps_a_copy() {
    let synced = content(SYNCED_BODY);
    let recorded = projection(SYNCED_BODY);
    let repo = conflicting(&Recorded::legacy(&synced, &recorded));

    let report = repo.resolving(1, Resolution::AcceptRemote);

    assert_pulled_keeping_a_copy(&repo, &report);
}

#[test]
fn a_blanked_local_hash_keeps_a_copy() {
    let recorded = projection(SYNCED_BODY);
    let repo = conflicting(&Recorded::Split {
        local: None,
        projection: Some(&recorded),
    });

    let report = repo.resolving(1, Resolution::AcceptRemote);

    assert_pulled_keeping_a_copy(&repo, &report);
}

#[test]
fn an_item_without_a_baseline_entry_keeps_a_copy() {
    let repo = conflicting(&Recorded::Nothing);

    let report = repo.resolving(1, Resolution::AcceptRemote);

    assert_pulled_keeping_a_copy(&repo, &report);
}

#[test]
fn a_clean_file_matching_its_recipe_one_baseline_keeps_no_copy() {
    let synced = content(SYNCED_BODY);
    let recorded = projection(SYNCED_BODY);
    let repo = Repo::with_item(
        Tracker::Linear,
        1,
        &synced,
        remote_moved(),
        &Recorded::split(&synced, &recorded),
    );

    let report = repo.sync_once();

    assert_eq!(
        outcome_of(&report, 1),
        (SyncState::RemotelyModified, Action::Pull)
    );
    assert!(matches!(row(&report, 1).outcome, ItemOutcome::Applied));
    assert_eq!(kept_edits(&report), None);
    assert_eq!(repo.kept_edits(), Vec::<PathBuf>::new());
}

fn assert_not_pulled_over(repo: &Repo, report: &RunReport, edit: &str) {
    let ItemOutcome::Failed(error) = &row(report, 1).outcome else {
        panic!("the pull is refused");
    };
    assert!(
        matches!(error, ApplyError::LocalChangedSinceClassified { .. }),
        "{error}"
    );
    assert_eq!(error.class(), FailureClass::Retryable);
    assert_eq!(repo.read(1), edit);
}

#[test]
fn a_file_edited_between_planning_and_apply_is_not_pulled_over() {
    let synced = content(SYNCED_BODY);
    let recorded = projection(SYNCED_BODY);
    let edit = content("Edited while the run was planning\n");
    let clean = Repo::with_item(
        Tracker::Linear,
        1,
        &synced,
        remote_moved(),
        &Recorded::split(&synced, &recorded),
    )
    .editing_when_locked(1, &edit);
    let before = clean.entry(1);

    let report = clean.sync_once();

    assert_not_pulled_over(&clean, &report, &edit);
    assert_eq!(clean.entry(1), before, "the baseline is unchanged");

    let resolved = conflicting(&Recorded::split(&synced, &recorded))
        .editing_when_locked(1, &edit);
    let before = resolved.entry(1);

    let report = resolved.resolving(1, Resolution::AcceptRemote);

    assert_not_pulled_over(&resolved, &report, &edit);
    assert_eq!(resolved.entry(1), before, "the baseline is unchanged");
}

#[test]
fn a_file_edited_while_its_edits_are_kept_is_not_pulled_over() {
    let synced = content(SYNCED_BODY);
    let recorded = projection(SYNCED_BODY);
    let edit = content("Edited while the copy was being kept\n");
    let repo = conflicting(&Recorded::split(&synced, &recorded))
        .editing_when_kept(1, &edit);

    let report = repo.resolving(1, Resolution::AcceptRemote);

    assert_not_pulled_over(&repo, &report, &edit);
    assert_eq!(kept_edits(&report), Some(PathBuf::from(KEPT_DIR)));
}

mod in_a_real_repository {
    use std::path::PathBuf;

    use vcs::tracking::FileTracking;
    use vcs::tracking::RepositoryTracking as _;
    use vcs_test_support::hermetic::Hermetic;
    use work::sync::Action;
    use work::sync::SyncState;
    use work_adapters::sync::run::ItemOutcome;

    use super::content;
    use super::projection;
    use super::remote_moved;
    use super::KEPT_DIR;
    use super::SYNCED_BODY;
    use crate::support::repo::outcome_of;
    use crate::support::repo::row;
    use crate::support::repo::work_item;
    use crate::support::repo::Recorded;
    use crate::support::repo::Repo;
    use crate::support::repo::Tracker;

    type TestError = Box<dyn std::error::Error>;

    /// Item 1, whose file the repository ignores, with only its status
    /// edited since it synced, while its remote moved.
    fn ignored_and_edited(synced: &str, recorded: &str) -> Repo {
        Repo::with_item(
            Tracker::Linear,
            1,
            &work_item(1, super::TITLE, "status: done\n", SYNCED_BODY),
            remote_moved(),
            &Recorded::split(synced, recorded),
        )
        .asking_the_repository()
    }

    fn assert_copied_but_still_pulled(repo: &Repo) {
        assert_eq!(
            vcs_adapters::InProcessTracking
                .file_tracking(&repo.root().join("0001.md")),
            FileTracking::Untracked,
            "the repository ignores the item"
        );

        let report = repo.sync_once();

        assert_eq!(
            outcome_of(&report, 1),
            (SyncState::RemotelyModified, Action::Pull)
        );
        assert!(matches!(row(&report, 1).outcome, ItemOutcome::Applied));
        assert_eq!(repo.kept_edits(), vec![PathBuf::from(KEPT_DIR)]);
    }

    #[test]
    fn an_ignored_edited_file_is_copied_but_still_planned_as_a_pull_under_git(
    ) -> Result<(), TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let (synced, recorded) =
            (content(SYNCED_BODY), projection(SYNCED_BODY));
        let repo = ignored_and_edited(&synced, &recorded);
        let env = Hermetic::rooted_at(repo.root())?;
        env.git(&["init", "--quiet"], repo.root())?;
        std::fs::write(repo.root().join(".gitignore"), "*.md\n")?;

        assert_copied_but_still_pulled(&repo);
        Ok(())
    }

    #[test]
    fn an_ignored_edited_file_is_copied_but_still_planned_as_a_pull_under_jj(
    ) -> Result<(), TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let (synced, recorded) =
            (content(SYNCED_BODY), projection(SYNCED_BODY));
        let repo = ignored_and_edited(&synced, &recorded);
        let env = Hermetic::rooted_at(repo.root())?;
        std::fs::write(repo.root().join(".gitignore"), "*.md\n")?;
        env.jj(&["git", "init", "--no-colocate"], repo.root())?;

        assert_copied_but_still_pulled(&repo);
        Ok(())
    }
}
