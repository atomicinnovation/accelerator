//! Learning the key each synced item's issue answers to now, before a
//! `{tracker}` re-key targets it.

use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker::TrackerError;
use tracker_test_support::Call;
use tracker_test_support::RecordingTracker;
use work::rekey::mapping::RefreshedKeys;
use work::rekey::outcome::NotRekeyed;
use work::tracker_key::TrackerKey;
use work_adapters::rekey::refresh::refresh_external_ids;
use work_adapters::sync::fetch::RetrievalStrategy;

type TestError = Box<dyn std::error::Error>;

fn id(raw: &str) -> ExternalId {
    ExternalId::new(raw.to_owned())
}

fn key(raw: &str) -> TrackerKey {
    TrackerKey::parse(raw).unwrap_or_else(|| unreachable!("a tracker key"))
}

fn issue(raw: &str) -> (ExternalId, RemoteIssue) {
    (
        id(raw),
        RemoteIssue {
            key: id(raw),
            updated: RemoteTimestamp::Reported("2026-10-01".to_owned()),
            body: String::new(),
        },
    )
}

fn holding(keys: &[&str]) -> RecordingTracker {
    RecordingTracker::holding(keys.iter().map(|raw| issue(raw)).collect())
}

fn retryable(detail: &str) -> TrackerError {
    TrackerError::Retryable {
        detail: detail.to_owned(),
    }
}

fn refreshed(
    keys: &[&str],
    tracker: &RecordingTracker,
    strategy: RetrievalStrategy,
) -> Result<RefreshedKeys, NotRekeyed> {
    let keys: Vec<ExternalId> = keys.iter().map(|raw| id(raw)).collect();
    refresh_external_ids(&keys, tracker, strategy)
}

fn located(tracker: &RecordingTracker) -> Vec<ExternalId> {
    tracker
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            Call::Locate { id } => Some(id),
            _ => None,
        })
        .collect()
}

fn assert_unreachable(
    outcome: &Result<RefreshedKeys, NotRekeyed>,
    detail: &str,
) {
    assert!(
        matches!(
            outcome,
            Err(NotRekeyed::TrackerUnreachable(reported))
                if reported.contains(detail)
        ),
        "{outcome:?}"
    );
}

#[test]
fn bulk_found_keys_are_unmoved_and_need_no_locate() -> Result<(), TestError> {
    let tracker = holding(&["PP-760"]);

    let keys = refreshed(&["PP-760"], &tracker, RetrievalStrategy::Bulk)?;

    assert_eq!(keys.current_key_for(&key("PP-760")), key("PP-760"));
    assert_eq!(located(&tracker), Vec::new());
    Ok(())
}

#[test]
fn bulk_absent_and_indeterminate_keys_are_located() -> Result<(), TestError> {
    let tracker = RecordingTracker::truncating(
        vec![issue("PP-760"), issue("PP-761")],
        vec![id("PP-761")],
    )
    .moving(&id("PP-760"), &id("ENG-42"))
    .moving(&id("PP-761"), &id("ENG-43"));

    let keys =
        refreshed(&["PP-760", "PP-761"], &tracker, RetrievalStrategy::Bulk)?;

    assert_eq!(located(&tracker), vec![id("PP-760"), id("PP-761")]);
    assert_eq!(keys.current_key_for(&key("PP-760")), key("ENG-42"));
    assert_eq!(keys.current_key_for(&key("PP-761")), key("ENG-43"));
    Ok(())
}

#[test]
fn a_located_move_yields_the_current_key() -> Result<(), TestError> {
    let tracker = holding(&["PP-760"]).moving(&id("PP-760"), &id("ENG-42"));

    let keys = refreshed(&["PP-760"], &tracker, RetrievalStrategy::Bulk)?;

    assert_eq!(keys.current_key_for(&key("PP-760")), key("ENG-42"));
    Ok(())
}

#[test]
fn a_case_only_difference_is_not_a_move() -> Result<(), TestError> {
    let tracker = holding(&["PP-760"]).moving(&id("pp-760"), &id("PP-760"));

    let keys = refreshed(&["pp-760"], &tracker, RetrievalStrategy::Bulk)?;

    assert_eq!(located(&tracker), vec![id("pp-760")]);
    assert_eq!(keys, RefreshedKeys::default());
    Ok(())
}

#[test]
fn a_not_found_key_keeps_its_stored_value() -> Result<(), TestError> {
    let tracker = holding(&[]).not_found(&id("PP-760"));

    let keys = refreshed(&["PP-760"], &tracker, RetrievalStrategy::Bulk)?;

    assert_eq!(keys.current_key_for(&key("PP-760")), key("PP-760"));
    Ok(())
}

#[test]
fn a_fetch_all_error_is_unreachable() {
    let tracker =
        holding(&["PP-760"]).failing_fetch_all(retryable("no credentials"));

    let outcome = refreshed(&["PP-760"], &tracker, RetrievalStrategy::Bulk);

    assert_unreachable(&outcome, "no credentials");
}

#[test]
fn a_failed_locate_after_a_bulk_read_is_unreachable() {
    let tracker =
        holding(&[]).failing_locate(&id("PP-760"), retryable("timeout"));

    let outcome = refreshed(&["PP-760"], &tracker, RetrievalStrategy::Bulk);

    assert_unreachable(&outcome, "timeout");
}

#[test]
fn a_total_transport_failure_is_unreachable() {
    let tracker = RecordingTracker::transient_keyed_read(Vec::new());

    let outcome =
        refreshed(&["PP-760", "PP-761"], &tracker, RetrievalStrategy::Bulk);

    assert_unreachable(&outcome, "PP-760");
    assert_eq!(located(&tracker), vec![id("PP-760")]);
}

#[test]
fn per_item_reads_locate_every_key() -> Result<(), TestError> {
    let tracker =
        holding(&["PP-760", "PP-761"]).moving(&id("PP-761"), &id("ENG-43"));

    let keys =
        refreshed(&["PP-760", "PP-761"], &tracker, RetrievalStrategy::PerItem)?;

    assert_eq!(
        tracker.calls(),
        vec![
            Call::Locate { id: id("PP-760") },
            Call::Locate { id: id("PP-761") },
        ]
    );
    assert_eq!(keys.current_key_for(&key("PP-761")), key("ENG-43"));
    Ok(())
}

#[test]
fn a_failed_per_item_locate_is_unreachable() {
    let tracker = holding(&["PP-760"])
        .failing_locate(&id("PP-760"), retryable("connection reset"));

    let outcome = refreshed(&["PP-760"], &tracker, RetrievalStrategy::PerItem);

    assert_unreachable(&outcome, "connection reset");
}
