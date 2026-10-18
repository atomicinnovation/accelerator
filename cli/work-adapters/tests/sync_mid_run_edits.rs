//! Runs during which a local file changes after the run has read it: the
//! edit is never absorbed into the baseline nor overwritten.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use work::sync::Action;
use work::sync::FieldDelivery;
use work::sync::SyncState;
use work_adapters::sync::apply::ApplyError;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::RunReport;

use crate::support::repo::issue;
use crate::support::repo::outcome_of;
use crate::support::repo::row;
use crate::support::repo::work_item;
use crate::support::repo::LastSynced;
use crate::support::repo::Recorded;
use crate::support::repo::Repo;
use crate::support::repo::Seed;
use crate::support::repo::Tracker;
use crate::support::repo::MOVED_STAMP;
use crate::support::repo::STAMP;

const TITLE: &str = "Add search";
const READY: &str = "status: ready\n";
const BODY: &str = "Original body\n";

fn linear(title: &str, body: &str) -> String {
    format!("{title}\n{body}")
}

fn item(index: usize, title: &str, body: &str) -> String {
    work_item(index, title, READY, body)
}

fn synced(index: usize) -> (String, String) {
    (item(index, TITLE, BODY), linear(TITLE, BODY))
}

fn changed_since_read(report: &RunReport, index: usize) -> bool {
    matches!(
        row(report, index).outcome,
        ItemOutcome::Failed(ApplyError::LocalChangedSinceClassified { .. })
    )
}

/// Item 1, holding `content` with its body edited since it synced
/// `recorded`.
fn edited_locally<'a>(
    content: &'a str,
    recorded: &'a (String, String),
) -> Seed<'a> {
    Seed {
        index: 1,
        content,
        remote: issue(1, &recorded.1, STAMP),
        recorded: Recorded::split(&recorded.0, &recorded.1),
        last_synced: LastSynced::BeforeItsLastEdit,
    }
}

#[test]
fn a_body_edited_while_its_title_is_sent_is_pushed_on_the_next_run() {
    let (content, projection) = synced(1);
    let repo = Repo::with_item(
        Tracker::Linear,
        1,
        &item(1, "Renamed", BODY),
        issue(1, &projection, STAMP),
        &Recorded::split(&content, &projection),
    )
    .editing_when_written(1, 1, &item(1, "Renamed", "Edited mid-run\n"));

    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::LocallyModified, Action::Retitle)
    );
    let next = repo.sync_once();

    assert_eq!(outcome_of(&next, 1).0, SyncState::LocallyModified);
    assert!(
        repo.remote(1).contains("Edited mid-run"),
        "{}",
        repo.remote(1)
    );
}

#[test]
fn a_body_edited_again_while_its_first_edit_is_sent_is_pushed_on_the_next_run()
{
    let recorded = synced(1);
    let content = item(1, TITLE, "Edited locally\n");
    let repo = Repo::with_items(
        Tracker::Linear,
        &[edited_locally(&content, &recorded)],
    )
    .editing_when_written(1, 1, &item(1, TITLE, "Edited again\n"));

    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (
            SyncState::LocallyModified,
            Action::Push(FieldDelivery::Withhold)
        )
    );
    let next = repo.sync_once();

    assert_eq!(outcome_of(&next, 1).0, SyncState::LocallyModified);
    assert!(
        repo.remote(1).contains("Edited again"),
        "{}",
        repo.remote(1)
    );
}

#[test]
fn a_pull_over_a_file_edited_after_the_run_read_it_writes_nothing() {
    let first = synced(1);
    let first_content = item(1, TITLE, "Edited locally\n");
    let second = synced(2);
    let edited = item(2, TITLE, "Edited mid-run\n");
    let repo = Repo::with_items(
        Tracker::Linear,
        &[
            edited_locally(&first_content, &first),
            Seed {
                index: 2,
                content: &second.0,
                remote: issue(2, &linear(TITLE, "Remote edit\n"), MOVED_STAMP),
                recorded: Recorded::split(&second.0, &second.1),
                last_synced: LastSynced::BeforeItsLastEdit,
            },
        ],
    )
    .editing_when_written(1, 2, &edited);

    let report = repo.sync_once();

    assert_eq!(
        outcome_of(&report, 2),
        (SyncState::RemotelyModified, Action::Pull)
    );
    assert!(changed_since_read(&report, 2));
    assert_eq!(repo.read(2), edited);
    assert_eq!(outcome_of(&repo.sync_once(), 2).0, SyncState::Conflict);
}

#[test]
fn a_baseline_record_over_a_file_edited_after_the_run_read_it_records_nothing()
{
    let first = synced(1);
    let first_content = item(1, TITLE, "Edited locally\n");
    let second = synced(2);
    let repo = Repo::with_items(
        Tracker::Linear,
        &[
            edited_locally(&first_content, &first),
            Seed {
                index: 2,
                content: &second.0,
                remote: issue(
                    2,
                    &linear("Renamed remotely", BODY),
                    MOVED_STAMP,
                ),
                recorded: Recorded::split(&second.0, &second.1),
                last_synced: LastSynced::AfterItsLastEdit,
            },
        ],
    )
    .editing_when_written(1, 2, &item(2, TITLE, "Edited mid-run\n"));
    let recorded = repo.entry(2);

    let report = repo.sync_once();

    assert_eq!(
        outcome_of(&report, 2),
        (SyncState::Synced, Action::RecordBaseline)
    );
    assert!(changed_since_read(&report, 2));
    assert_eq!(repo.entry(2), recorded);
}
