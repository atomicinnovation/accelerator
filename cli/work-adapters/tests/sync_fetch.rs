//! The fetch shell's unit-level branches: the `fetch_all`-error branch, the
//! two-tier read rule, presence mapping, and dirtiness wiring.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;
use std::path::PathBuf;

use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker_test_support::RecordingTracker;
use work::sync::Dirtiness;
use work::sync::RemotePresence;
use work_adapters::sync::baseline::Baseline;
use work_adapters::sync::digest::LazyItemDigests;
use work_adapters::sync::fetch;
use work_adapters::sync::fetch::LocalItem;
use work_adapters::sync::fetch::RetrievalStrategy;
use work_adapters::sync::fetch::WorkingCopyStatus;

struct AlwaysClean;
impl WorkingCopyStatus for AlwaysClean {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        Dirtiness::Clean
    }
}

struct AlwaysDirty;
impl WorkingCopyStatus for AlwaysDirty {
    fn is_dirty(&self, _path: &Path) -> Dirtiness {
        Dirtiness::Dirty
    }
}

fn item(id: &str, external_id: Option<&str>) -> LocalItem {
    LocalItem {
        id: id.to_owned(),
        path: PathBuf::from(format!("/items/{id}.md")),
        external_id: external_id.map(|raw| ExternalId::new(raw.to_owned())),
    }
}

/// `RecordingTracker::holding` never fails `fetch_all`, so this pins the
/// partition for a tracker that knows nothing: every id comes back absent
/// — a complete but empty catalogue — rather than indeterminate. A genuine
/// pre-flight failure has no seam here and is covered by `work-cli`'s
/// command-level tests.
#[test]
fn a_no_entry_item_falls_back_to_the_document_timestamp_for_its_gate() {
    let tracker = RecordingTracker::holding(Vec::new());
    let (baseline, _) = Baseline::read(Some(
        r#"{"timestamp":900,"items":{"0001":{"remote_updated_at":"x","remote_hash":"h","local_hash":"h","local_synced_at":500}}}"#,
    ));
    let items = vec![item("0001", Some("ENG-1")), item("0002", Some("ENG-2"))];
    let facts = fetch::gather(
        &items,
        &baseline,
        &tracker,
        &AlwaysClean,
        RetrievalStrategy::Bulk,
    );
    let digests: Vec<LazyItemDigests<'_>> = items
        .iter()
        .map(|entry| LazyItemDigests::new(&entry.path, None))
        .collect();

    let inputs = facts.plan_inputs(&items, &digests, &baseline);
    let watermark = |id: &str| {
        inputs
            .iter()
            .find(|input| input.id == id)
            .expect("input present")
            .baseline_timestamp
    };

    assert_eq!(
        watermark("0001"),
        500,
        "an item with a baseline entry uses that entry's own watermark"
    );
    assert_eq!(
        watermark("0002"),
        900,
        "an item with no entry falls back to the document timestamp"
    );
}

#[test]
fn a_complete_but_empty_catalogue_reports_absence_not_indeterminate() {
    let tracker = RecordingTracker::holding(Vec::new());
    let baseline = Baseline::read(None).0;
    let items = vec![item("0001", Some("ENG-1")), item("0002", Some("ENG-2"))];

    let facts = fetch::gather(
        &items,
        &baseline,
        &tracker,
        &AlwaysClean,
        RetrievalStrategy::Bulk,
    );

    assert!(facts.read_failure.is_none());
    assert_eq!(
        facts.per_id.get("0001").map(|(remote, _)| remote.presence),
        Some(RemotePresence::Absent)
    );
    assert_eq!(
        facts.per_id.get("0002").map(|(remote, _)| remote.presence),
        Some(RemotePresence::Absent)
    );
}

#[test]
fn a_stamp_that_proves_unchanged_costs_no_show() {
    let id = ExternalId::new("ENG-1".to_owned());
    let issue = RemoteIssue {
        key: ExternalId::new("ENG-1".to_owned()),
        updated: RemoteTimestamp::Reported("2026-06-01".to_owned()),
        body: "Title\nBody\n".to_owned(),
    };
    let tracker = RecordingTracker::holding(vec![(id, issue)]);
    let mut baseline = Baseline::read(None).0;
    baseline.set(
        "0001",
        work_adapters::sync::baseline::Entry {
            remote_updated_at: RemoteTimestamp::Reported(
                "2026-06-01".to_owned(),
            ),
            remote_hash: "h".to_owned(),
            local_hash: "h".to_owned(),
            local_synced_at: 0,
        },
    );
    let items = vec![item("0001", Some("ENG-1"))];

    let facts = fetch::gather(
        &items,
        &baseline,
        &tracker,
        &AlwaysClean,
        RetrievalStrategy::Bulk,
    );

    assert!(!tracker
        .calls()
        .iter()
        .any(|call| matches!(call, tracker_test_support::Call::Show { .. })));
    assert_eq!(
        facts
            .per_id
            .get("0001")
            .map(|(remote, _)| remote.body.clone()),
        Some(None)
    );
}

#[test]
fn a_stamp_that_does_not_prove_unchanged_costs_exactly_one_show() {
    let id = ExternalId::new("ENG-1".to_owned());
    let issue = RemoteIssue {
        key: ExternalId::new("ENG-1".to_owned()),
        updated: RemoteTimestamp::Reported("2026-07-01".to_owned()),
        body: "Title\nNew body\n".to_owned(),
    };
    let tracker = RecordingTracker::holding(vec![(id, issue)]);
    let mut baseline = Baseline::read(None).0;
    baseline.set(
        "0001",
        work_adapters::sync::baseline::Entry {
            remote_updated_at: RemoteTimestamp::Reported(
                "2026-06-01".to_owned(),
            ),
            remote_hash: "h".to_owned(),
            local_hash: "h".to_owned(),
            local_synced_at: 0,
        },
    );
    let items = vec![item("0001", Some("ENG-1"))];

    let facts = fetch::gather(
        &items,
        &baseline,
        &tracker,
        &AlwaysClean,
        RetrievalStrategy::Bulk,
    );

    let show_calls = tracker
        .calls()
        .iter()
        .filter(|call| matches!(call, tracker_test_support::Call::Show { .. }))
        .count();
    assert_eq!(show_calls, 1);
    assert_eq!(
        facts
            .per_id
            .get("0001")
            .map(|(remote, _)| remote.body.clone()),
        Some(Some("Title\nNew body\n".to_owned()))
    );
}

#[test]
fn absent_and_indeterminate_partition_correctly() {
    let known_id = ExternalId::new("ENG-1".to_owned());
    let unseen_id = ExternalId::new("ENG-2".to_owned());
    let issue = RemoteIssue {
        key: ExternalId::new("ENG-1".to_owned()),
        updated: RemoteTimestamp::NotReported,
        body: String::new(),
    };
    let tracker =
        RecordingTracker::truncating(vec![(known_id, issue)], vec![unseen_id]);
    let baseline = Baseline::read(None).0;
    let items = vec![
        item("0001", Some("ENG-1")),
        item("0002", Some("ENG-2")),
        item("0003", Some("ENG-3")),
    ];

    let facts = fetch::gather(
        &items,
        &baseline,
        &tracker,
        &AlwaysClean,
        RetrievalStrategy::Bulk,
    );

    assert_eq!(
        facts.per_id.get("0002").map(|(r, _)| r.presence),
        Some(RemotePresence::Indeterminate)
    );
    assert_eq!(
        facts.per_id.get("0003").map(|(r, _)| r.presence),
        Some(RemotePresence::Absent)
    );
}

#[test]
fn per_item_strategy_calls_show_for_every_present_id_and_never_fetch_all() {
    let id = ExternalId::new("ENG-1".to_owned());
    let issue = RemoteIssue {
        key: ExternalId::new("ENG-1".to_owned()),
        updated: RemoteTimestamp::Reported("x".to_owned()),
        body: "Title\nBody\n".to_owned(),
    };
    let tracker = RecordingTracker::holding(vec![(id, issue)]);
    let baseline = Baseline::read(None).0;
    let items = vec![item("0001", Some("ENG-1"))];

    let _ = fetch::gather(
        &items,
        &baseline,
        &tracker,
        &AlwaysClean,
        RetrievalStrategy::PerItem,
    );

    let calls = tracker.calls();
    assert!(calls
        .iter()
        .any(|call| matches!(call, tracker_test_support::Call::Show { .. })));
    assert!(!calls.iter().any(|call| matches!(
        call,
        tracker_test_support::Call::FetchAll { .. }
    )));
}

#[test]
fn a_status_probe_failure_yields_unknown_which_decides_as_dirty() {
    struct AlwaysUnknown;
    impl WorkingCopyStatus for AlwaysUnknown {
        fn is_dirty(&self, _path: &Path) -> Dirtiness {
            Dirtiness::Unknown
        }
    }

    let tracker = RecordingTracker::holding(Vec::new());
    let baseline = Baseline::read(None).0;
    let items = vec![item("0001", Some("ENG-1"))];

    let facts = fetch::gather(
        &items,
        &baseline,
        &tracker,
        &AlwaysUnknown,
        RetrievalStrategy::Bulk,
    );

    assert_eq!(
        facts.per_id.get("0001").map(|(_, dirty)| *dirty),
        Some(Dirtiness::Unknown)
    );
}

#[test]
fn an_item_with_no_external_id_still_gets_a_facts_entry() {
    let tracker = RecordingTracker::holding(Vec::new());
    let baseline = Baseline::read(None).0;
    let items = vec![item("0001", None)];

    let facts = fetch::gather(
        &items,
        &baseline,
        &tracker,
        &AlwaysDirty,
        RetrievalStrategy::Bulk,
    );

    assert!(facts.per_id.contains_key("0001"));
}

fn key(raw: &str) -> ExternalId {
    ExternalId::new(raw.to_owned())
}

fn held(raw: &str, stamp: &str) -> (ExternalId, RemoteIssue) {
    (
        key(raw),
        RemoteIssue {
            key: key(raw),
            updated: RemoteTimestamp::Reported(stamp.to_owned()),
            body: "Title\nBody\n".to_owned(),
        },
    )
}

fn gathered(
    tracker: &RecordingTracker,
    items: &[LocalItem],
    strategy: RetrievalStrategy,
) -> fetch::GatheredFacts {
    fetch::gather(
        items,
        &Baseline::read(None).0,
        tracker,
        &AlwaysClean,
        strategy,
    )
}

#[test]
fn an_id_absent_from_the_bulk_read_is_located_and_reported_moved() {
    let tracker = RecordingTracker::holding(vec![held("PP-760", "2026-07-01")])
        .moving(&key("PP-760"), &key("ENG-42"));
    let items = vec![item("0230", Some("PP-760"))];

    let facts = gathered(&tracker, &items, RetrievalStrategy::Bulk);

    assert_eq!(
        facts.identity.get("0230"),
        Some(&fetch::IdentityObservation::Moved {
            old: key("PP-760"),
            new: key("ENG-42"),
        })
    );
    let (remote, _) = facts.per_id.get("0230").expect("a fact");
    assert_eq!(remote.presence, RemotePresence::Present);
    assert_eq!(remote.body.as_deref(), Some("Title\nBody\n"));
}

#[test]
fn a_show_answering_another_key_after_the_bulk_read_is_a_move() {
    let tracker = RecordingTracker::holding(vec![held("PP-760", "2026-07-01")])
        .showing_under(&key("PP-760"), &key("ENG-42"));
    let items = vec![item("0230", Some("PP-760"))];

    let facts = gathered(&tracker, &items, RetrievalStrategy::Bulk);

    assert_eq!(
        facts.identity.get("0230"),
        Some(&fetch::IdentityObservation::Moved {
            old: key("PP-760"),
            new: key("ENG-42"),
        })
    );
}

#[test]
fn a_key_answered_in_another_case_is_not_a_move() {
    let tracker = RecordingTracker::holding(vec![held("PP-760", "2026-07-01")])
        .showing_under(&key("PP-760"), &key("pp-760"));
    let items = vec![item("0230", Some("PP-760"))];

    let facts = gathered(&tracker, &items, RetrievalStrategy::Bulk);

    assert!(facts.identity.is_empty(), "{:?}", facts.identity);
}

#[test]
fn a_per_item_read_answering_another_key_is_a_move() {
    let tracker = RecordingTracker::holding(vec![held("PP-760", "2026-07-01")])
        .moving(&key("PP-760"), &key("ENG-42"));
    let items = vec![item("0230", Some("PP-760"))];

    let facts = gathered(&tracker, &items, RetrievalStrategy::PerItem);

    assert_eq!(
        facts.identity.get("0230"),
        Some(&fetch::IdentityObservation::Moved {
            old: key("PP-760"),
            new: key("ENG-42"),
        })
    );
}

#[test]
fn an_id_the_tracker_cannot_find_is_reported_not_found_and_absent() {
    let tracker =
        RecordingTracker::holding(Vec::new()).not_found(&key("PP-76"));
    let items = vec![item("PP-76", Some("PP-76"))];

    let facts = gathered(&tracker, &items, RetrievalStrategy::Bulk);

    assert_eq!(
        facts.identity.get("PP-76"),
        Some(&fetch::IdentityObservation::NotFound { key: key("PP-76") })
    );
    assert_eq!(
        facts.per_id.get("PP-76").map(|(remote, _)| remote.presence),
        Some(RemotePresence::Absent)
    );
}

#[test]
fn a_failed_locate_leaves_the_bulk_reads_answer() {
    let tracker = RecordingTracker::holding(Vec::new()).failing_locate(
        &key("PP-76"),
        tracker::TrackerError::Retryable {
            detail: "down".to_owned(),
        },
    );
    let items = vec![item("PP-76", Some("PP-76"))];

    let facts = gathered(&tracker, &items, RetrievalStrategy::Bulk);

    assert!(facts.identity.is_empty());
    assert_eq!(
        facts.per_id.get("PP-76").map(|(remote, _)| remote.presence),
        Some(RemotePresence::Absent)
    );
}

#[test]
fn an_indeterminate_id_is_located() {
    let tracker = RecordingTracker::truncating(Vec::new(), vec![key("PP-760")])
        .not_found(&key("PP-760"));
    let items = vec![item("0230", Some("PP-760"))];

    let facts = gathered(&tracker, &items, RetrievalStrategy::Bulk);

    assert_eq!(
        facts.identity.get("0230"),
        Some(&fetch::IdentityObservation::NotFound { key: key("PP-760") })
    );
}
