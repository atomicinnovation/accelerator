//! Trackers that answer body rules as a shipped client does, and the
//! assertions every baseline writer's tests share.
#![allow(dead_code)]

pub mod repo;

use corpus_adapters::CommonMarkBody;
use jira_client::JiraBodyDialect;
use tracker::BodyDialect;
use tracker::ExternalId;
use tracker::RemoteIssue;
use tracker::RemoteTimestamp;
use tracker_test_support::RecordingTracker;
use work_adapters::sync::baseline::Baseline;
use work_adapters::sync::baseline::Entry;
use work_adapters::sync::baseline::SyncStamps;
use work_adapters::sync::baseline_digester::BaselineDigester;

/// A recording tracker holding `issues` under Jira's body rules.
pub fn jira_recording_tracker(
    issues: Vec<(ExternalId, RemoteIssue)>,
) -> RecordingTracker {
    RecordingTracker::holding(issues).with_dialect(Box::new(JiraBodyDialect))
}

/// Asserts `entry` is a recipe-1 entry recording both recipes' digests of
/// `local_content` and `remote_projection`.
///
/// # Panics
///
/// When the entry is absent, not recipe 1, or records other digests.
#[allow(clippy::panic, clippy::expect_used)]
pub fn assert_recipe_one_entry(
    entry: Option<&Entry>,
    dialect: &dyn BodyDialect,
    local_content: &str,
    remote_projection: Option<&str>,
) {
    let recorded = BaselineDigester::new(dialect, &CommonMarkBody)
        .digests(local_content, remote_projection)
        .expect("the local content is well formed");
    let Some(Entry::Split { legacy, split, .. }) = entry else {
        panic!("expected a recipe-1 entry, found {entry:?}");
    };
    assert_eq!(split, &recorded.split, "the split fingerprints");
    assert_eq!(legacy, &recorded.legacy, "the legacy hashes");
}

/// `id`'s recipe-1 entry as a `last-sync.json` item fragment, recording
/// `local_content` and `remote_projection` under `dialect`'s body rules.
///
/// # Panics
///
/// When `local_content` is not a well-formed work item.
#[allow(clippy::expect_used)]
pub fn split_entry(
    id: &str,
    dialect: &dyn BodyDialect,
    local_content: &str,
    remote_projection: &str,
    remote_updated_at: &str,
) -> String {
    let fingerprints = BaselineDigester::new(dialect, &CommonMarkBody)
        .digests(local_content, Some(remote_projection))
        .expect("the local content is well formed");
    entry_fragment(
        id,
        Entry::recorded(
            SyncStamps {
                remote_updated_at: RemoteTimestamp::Reported(
                    remote_updated_at.to_owned(),
                ),
                local_synced_at: 0,
            },
            fingerprints,
        ),
    )
}

/// `entry` as `id`'s `last-sync.json` item fragment.
///
/// # Panics
///
/// Never: one rendered entry always has the sliced shape.
#[allow(clippy::expect_used)]
pub fn entry_fragment(id: &str, entry: Entry) -> String {
    let mut baseline = Baseline::read(None).0;
    baseline.set(id, entry);
    baseline
        .render()
        .trim_end()
        .strip_prefix("{\"timestamp\":0,\"items\":{")
        .and_then(|items| items.strip_suffix("}}"))
        .expect("one rendered entry")
        .to_owned()
}
