//! The planner's branch table, at unit level with no fake tracker needed:
//! `needs_body_read`'s partition, the `RemotePresence` mapping, resolution
//! application over `Prompt`, and `Noop` suppression.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;

use tracker::ExternalId;
use tracker::RemoteTimestamp;
use work::sync::plan;
use work::sync::BaselineEntry;
use work::sync::BaselineFingerprints;
use work::sync::BodyEquivalence;
use work::sync::FieldDelivery;
use work::sync::FieldsComparison;
use work::sync::ItemComparison;
use work::sync::ItemDigests;
use work::sync::LegacyHashes;
use work::sync::LocalFingerprints;
use work::sync::PlanInput;
use work::sync::PullResidue;
use work::sync::RemoteFacts;
use work::sync::RemoteFingerprints;
use work::sync::RemotePresence;
use work::sync::Resolution;
use work::sync::SplitFingerprints;
use work::sync::SyncDirection;
use work::sync::SyncState;

struct NeverCalled;

impl ItemDigests for NeverCalled {
    fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
        panic!("mtime should not be called for a presence-absent item")
    }

    fn legacy_local(&self) -> Result<String, kernel::Error> {
        panic!("legacy_local should not be called for a presence-absent item")
    }

    fn legacy_remote(&self) -> Result<Option<String>, kernel::Error> {
        panic!("legacy_remote should not be called for a presence-absent item")
    }

    fn local(&self) -> Result<LocalFingerprints, kernel::Error> {
        panic!("local should not be called for a presence-absent item")
    }

    fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error> {
        panic!("remote should not be called for a presence-absent item")
    }
}

/// Two sides holding no residue whose bodies differ and whose titles
/// agree.
struct DifferingBodies;

impl ItemComparison for DifferingBodies {
    fn pull_residue(&self) -> Option<PullResidue> {
        None
    }

    fn body_equivalence(&self) -> BodyEquivalence {
        BodyEquivalence::Different
    }

    fn fields_comparison(&self) -> FieldsComparison {
        FieldsComparison::Same
    }
}

fn legacy(local: Option<&str>, remote: Option<&str>) -> BaselineFingerprints {
    BaselineFingerprints::Legacy(LegacyHashes {
        local: local.map(str::to_owned),
        remote: remote.map(str::to_owned),
    })
}

fn agreeing_split() -> BaselineFingerprints {
    BaselineFingerprints::Split(SplitFingerprints {
        local: Some(local_fingerprints("h")),
        remote: Some(remote_fingerprints("h")),
    })
}

fn local_fingerprints(hash: &str) -> LocalFingerprints {
    LocalFingerprints {
        frontmatter: hash.to_owned(),
        fields: hash.to_owned(),
        body: hash.to_owned(),
    }
}

fn remote_fingerprints(hash: &str) -> RemoteFingerprints {
    RemoteFingerprints {
        body: hash.to_owned(),
        fields: hash.to_owned(),
    }
}

const fn empty_baseline() -> BaselineEntry<'static> {
    BaselineEntry {
        remote_updated_at: &RemoteTimestamp::NotReported,
        fingerprints: BaselineFingerprints::Legacy(LegacyHashes {
            local: None,
            remote: None,
        }),
    }
}

#[test]
fn needs_body_read_skips_a_stamp_that_proves_unchanged() {
    let baseline_updated = RemoteTimestamp::Reported("2026-01-01".to_owned());
    let subject_updated = RemoteTimestamp::Reported("2026-01-01".to_owned());
    let digests = NeverCalled;
    let id = ExternalId::new("ENG-1".to_owned());
    let baseline = BaselineEntry {
        remote_updated_at: &baseline_updated,
        fingerprints: legacy(Some("h"), Some("h")),
    };
    let item = PlanInput {
        id: "0001".to_owned(),
        external_id: Some(&id),
        facts: RemoteFacts {
            presence: RemotePresence::Present,
            remote_updated: &subject_updated,
        },
        dirty: work::sync::Dirtiness::Clean,
        baseline,
        baseline_timestamp: 0,
        digests: &digests,
        comparison: &DifferingBodies,
    };

    assert_eq!(plan::needs_body_read(&[item]), Vec::new());
}

#[test]
fn needs_body_read_includes_a_stamp_that_does_not_prove_unchanged() {
    let baseline_updated = RemoteTimestamp::Reported("2026-01-01".to_owned());
    let subject_updated = RemoteTimestamp::Reported("2026-02-01".to_owned());
    let digests = NeverCalled;
    let id = ExternalId::new("ENG-1".to_owned());
    let item = PlanInput {
        id: "0001".to_owned(),
        external_id: Some(&id),
        facts: RemoteFacts {
            presence: RemotePresence::Present,
            remote_updated: &subject_updated,
        },
        dirty: work::sync::Dirtiness::Clean,
        baseline: BaselineEntry {
            remote_updated_at: &baseline_updated,
            fingerprints: legacy(Some("h"), Some("h")),
        },
        baseline_timestamp: 0,
        digests: &digests,
        comparison: &DifferingBodies,
    };

    assert_eq!(plan::needs_body_read(&[item]), vec![id]);
}

#[test]
fn needs_body_read_excludes_absent_and_indeterminate_items() {
    let updated = RemoteTimestamp::NotReported;
    let digests = NeverCalled;
    let absent_id = ExternalId::new("ENG-2".to_owned());
    let indeterminate_id = ExternalId::new("ENG-3".to_owned());
    let items = [
        PlanInput {
            id: "0002".to_owned(),
            external_id: Some(&absent_id),
            facts: RemoteFacts {
                presence: RemotePresence::Absent,
                remote_updated: &updated,
            },
            dirty: work::sync::Dirtiness::Clean,
            baseline: empty_baseline(),
            baseline_timestamp: 0,
            digests: &digests,
            comparison: &DifferingBodies,
        },
        PlanInput {
            id: "0003".to_owned(),
            external_id: Some(&indeterminate_id),
            facts: RemoteFacts {
                presence: RemotePresence::Indeterminate,
                remote_updated: &updated,
            },
            dirty: work::sync::Dirtiness::Clean,
            baseline: empty_baseline(),
            baseline_timestamp: 0,
            digests: &digests,
            comparison: &DifferingBodies,
        },
    ];

    assert_eq!(plan::needs_body_read(&items), Vec::new());
}

fn digests_agreeing_with(hash: &'static str) -> impl ItemDigests {
    struct Agreeing(&'static str);
    impl ItemDigests for Agreeing {
        fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
            Ok(None)
        }

        fn legacy_local(&self) -> Result<String, kernel::Error> {
            Ok(self.0.to_owned())
        }

        fn legacy_remote(&self) -> Result<Option<String>, kernel::Error> {
            Ok(Some(self.0.to_owned()))
        }

        fn local(&self) -> Result<LocalFingerprints, kernel::Error> {
            Ok(local_fingerprints(self.0))
        }

        fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error> {
            Ok(Some(remote_fingerprints(self.0)))
        }
    }
    Agreeing(hash)
}

#[test]
fn an_absent_item_plans_as_remote_absent_and_noop() {
    let updated = RemoteTimestamp::NotReported;
    let digests = NeverCalled;
    let id = ExternalId::new("ENG-4".to_owned());
    let item = PlanInput {
        id: "0004".to_owned(),
        external_id: Some(&id),
        facts: RemoteFacts {
            presence: RemotePresence::Absent,
            remote_updated: &updated,
        },
        dirty: work::sync::Dirtiness::Clean,
        baseline: empty_baseline(),
        baseline_timestamp: 0,
        digests: &digests,
        comparison: &DifferingBodies,
    };

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &BTreeMap::new());

    assert_eq!(result.actions.len(), 1);
    assert_eq!(result.actions[0].state(), SyncState::RemoteAbsent);
    assert_eq!(result.actions[0].action(), work::sync::Action::Noop);
}

#[test]
fn an_indeterminate_item_plans_as_indeterminate_and_noop() {
    let updated = RemoteTimestamp::NotReported;
    let digests = NeverCalled;
    let id = ExternalId::new("ENG-5".to_owned());
    let item = PlanInput {
        id: "0005".to_owned(),
        external_id: Some(&id),
        facts: RemoteFacts {
            presence: RemotePresence::Indeterminate,
            remote_updated: &updated,
        },
        dirty: work::sync::Dirtiness::Clean,
        baseline: empty_baseline(),
        baseline_timestamp: 0,
        digests: &digests,
        comparison: &DifferingBodies,
    };

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &BTreeMap::new());

    assert_eq!(result.actions.len(), 1);
    assert_eq!(result.actions[0].state(), SyncState::Indeterminate);
    assert_eq!(result.actions[0].action(), work::sync::Action::Noop);
}

fn conflicting_item<'a>(
    id: &'a str,
    external_id: &'a ExternalId,
    updated: &'a RemoteTimestamp,
    baseline_updated: &'a RemoteTimestamp,
    digests: &'a dyn ItemDigests,
) -> PlanInput<'a> {
    PlanInput {
        id: id.to_owned(),
        external_id: Some(external_id),
        facts: RemoteFacts {
            presence: RemotePresence::Present,
            remote_updated: updated,
        },
        dirty: work::sync::Dirtiness::Clean,
        baseline: BaselineEntry {
            remote_updated_at: baseline_updated,
            fingerprints: legacy(Some("stale"), Some("stale")),
        },
        baseline_timestamp: 0,
        digests,
        comparison: &DifferingBodies,
    }
}

#[test]
fn a_resolution_of_accept_remote_turns_a_prompt_into_a_pull() {
    let baseline_updated = RemoteTimestamp::Reported("old".to_owned());
    let subject_updated = RemoteTimestamp::Reported("new".to_owned());
    let digests = digests_agreeing_with("does-not-matter");
    let id = ExternalId::new("ENG-6".to_owned());
    let item = conflicting_item(
        "0006",
        &id,
        &subject_updated,
        &baseline_updated,
        &digests,
    );
    let mut resolutions = BTreeMap::new();
    resolutions.insert("0006".to_owned(), Resolution::AcceptRemote);

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &resolutions);

    assert_eq!(result.actions[0].state(), SyncState::Conflict);
    assert_eq!(result.actions[0].action(), work::sync::Action::Pull);
}

fn resolved_conflict(resolution: Resolution) -> work::sync::SyncPlan {
    let baseline_updated = RemoteTimestamp::Reported("old".to_owned());
    let subject_updated = RemoteTimestamp::Reported("new".to_owned());
    let digests = digests_agreeing_with("does-not-matter");
    let id = ExternalId::new("ENG-7".to_owned());
    let item = conflicting_item(
        "0007",
        &id,
        &subject_updated,
        &baseline_updated,
        &digests,
    );
    let resolutions = BTreeMap::from([("0007".to_owned(), resolution)]);

    plan::plan(&[item], SyncDirection::Bidirectional, &resolutions)
}

#[test]
fn a_resolved_local_push_is_an_overwrite() {
    let result = resolved_conflict(Resolution::PushLocal);

    assert_eq!(result.actions[0].action(), work::sync::Action::Overwrite);
}

#[test]
fn a_resolved_local_overwrite_counts_against_max_pushes() {
    let result = resolved_conflict(Resolution::PushLocal);

    assert_eq!(result.push_count(), 1);
}

#[test]
fn only_a_push_a_retitle_or_an_overwrite_sends_to_the_tracker() {
    use work::sync::Action;

    let sending: Vec<Action> = [
        Action::Push(FieldDelivery::Send),
        Action::Push(FieldDelivery::Withhold),
        Action::Retitle,
        Action::Overwrite,
        Action::Pull,
        Action::SkipConflict,
        Action::SkipDirty,
        Action::Prompt,
        Action::Noop,
        Action::RecordBaseline,
        Action::CreateFromRemote,
        Action::CreateFromLocal,
    ]
    .into_iter()
    .filter(|action| action.sends_to_tracker())
    .collect();

    assert_eq!(
        sending,
        [
            Action::Push(FieldDelivery::Send),
            Action::Push(FieldDelivery::Withhold),
            Action::Retitle,
            Action::Overwrite,
        ]
    );
}

#[test]
fn a_retitle_counts_against_max_pushes() {
    let retitling = work::sync::SyncPlan {
        actions: vec![work::sync::PlannedAction::new(
            "0013".to_owned(),
            SyncState::LocallyModified,
            work::sync::Action::Retitle,
        )],
        unclassifiable: Vec::new(),
    };

    assert_eq!(retitling.push_count(), 1);
}

#[test]
fn a_resolution_of_skip_leaves_the_prompt_unresolved() {
    let baseline_updated = RemoteTimestamp::Reported("old".to_owned());
    let subject_updated = RemoteTimestamp::Reported("new".to_owned());
    let digests = digests_agreeing_with("does-not-matter");
    let id = ExternalId::new("ENG-8".to_owned());
    let item = conflicting_item(
        "0008",
        &id,
        &subject_updated,
        &baseline_updated,
        &digests,
    );
    let mut resolutions = BTreeMap::new();
    resolutions.insert("0008".to_owned(), Resolution::Skip);

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &resolutions);

    assert_eq!(result.actions[0].action(), work::sync::Action::Prompt);
}

#[test]
fn a_stale_resolution_naming_an_id_that_did_not_prompt_is_inert() {
    let updated = RemoteTimestamp::Reported("same".to_owned());
    let digests = digests_agreeing_with("h");
    let id = ExternalId::new("ENG-9".to_owned());
    let item = PlanInput {
        id: "0009".to_owned(),
        external_id: Some(&id),
        facts: RemoteFacts {
            presence: RemotePresence::Present,
            remote_updated: &updated,
        },
        dirty: work::sync::Dirtiness::Clean,
        baseline: BaselineEntry {
            remote_updated_at: &updated,
            fingerprints: agreeing_split(),
        },
        baseline_timestamp: 0,
        digests: &digests,
        comparison: &DifferingBodies,
    };
    let mut resolutions = BTreeMap::new();
    resolutions.insert("0009".to_owned(), Resolution::AcceptRemote);

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &resolutions);

    assert_eq!(result.actions[0].state(), SyncState::Synced);
    assert_eq!(result.actions[0].action(), work::sync::Action::Noop);
}

#[test]
fn a_local_resolution_on_an_item_that_did_not_prompt_stays_inert() {
    let updated = RemoteTimestamp::Reported("old".to_owned());
    let moved = RemoteTimestamp::Reported("new".to_owned());
    let digests = digests_agreeing_with("h");
    let id = ExternalId::new("ENG-11".to_owned());
    let item = PlanInput {
        id: "0011".to_owned(),
        external_id: Some(&id),
        facts: RemoteFacts {
            presence: RemotePresence::Present,
            remote_updated: &moved,
        },
        dirty: work::sync::Dirtiness::Clean,
        baseline: BaselineEntry {
            remote_updated_at: &updated,
            fingerprints: legacy(Some("h"), Some("stale")),
        },
        baseline_timestamp: 0,
        digests: &digests,
        comparison: &DifferingBodies,
    };
    let resolutions =
        BTreeMap::from([("0011".to_owned(), Resolution::PushLocal)]);

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &resolutions);

    assert_eq!(result.actions[0].state(), SyncState::RemotelyModified);
    assert_eq!(result.actions[0].action(), work::sync::Action::Pull);
}

#[test]
fn pull_and_push_counts_reflect_only_their_own_action() {
    let updated = RemoteTimestamp::Reported("same".to_owned());
    let digests = digests_agreeing_with("h");
    let id = ExternalId::new("ENG-10".to_owned());
    let synced_item = PlanInput {
        id: "0010".to_owned(),
        external_id: Some(&id),
        facts: RemoteFacts {
            presence: RemotePresence::Present,
            remote_updated: &updated,
        },
        dirty: work::sync::Dirtiness::Clean,
        baseline: BaselineEntry {
            remote_updated_at: &updated,
            fingerprints: agreeing_split(),
        },
        baseline_timestamp: 0,
        digests: &digests,
        comparison: &DifferingBodies,
    };

    let result = plan::plan(
        &[synced_item],
        SyncDirection::Bidirectional,
        &BTreeMap::new(),
    );

    assert_eq!(result.pull_count(), 0);
    assert_eq!(result.push_count(), 0);
}

#[test]
fn a_synced_legacy_entry_plans_a_baseline_record_in_every_direction() {
    let updated = RemoteTimestamp::Reported("same".to_owned());
    let digests = digests_agreeing_with("h");
    let id = ExternalId::new("ENG-12".to_owned());
    for direction in [
        SyncDirection::Bidirectional,
        SyncDirection::PushOnly,
        SyncDirection::PullOnly,
    ] {
        let item = PlanInput {
            id: "0012".to_owned(),
            external_id: Some(&id),
            facts: RemoteFacts {
                presence: RemotePresence::Present,
                remote_updated: &updated,
            },
            dirty: work::sync::Dirtiness::Dirty,
            baseline: BaselineEntry {
                remote_updated_at: &updated,
                fingerprints: legacy(Some("h"), Some("h")),
            },
            baseline_timestamp: 0,
            digests: &digests,
            comparison: &DifferingBodies,
        };

        let result = plan::plan(&[item], direction, &BTreeMap::new());

        assert_eq!(result.actions[0].state(), SyncState::Synced);
        assert_eq!(
            result.actions[0].action(),
            work::sync::Action::RecordBaseline,
            "{direction:?}"
        );
    }
}

struct UnreadableSinceWatermark;

impl ItemDigests for UnreadableSinceWatermark {
    fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
        Ok(Some(u64::MAX))
    }

    fn legacy_local(&self) -> Result<String, kernel::Error> {
        Err(kernel::Error::Failed("the file vanished".to_owned()))
    }

    fn legacy_remote(&self) -> Result<Option<String>, kernel::Error> {
        Err(kernel::Error::Failed("the file vanished".to_owned()))
    }

    fn local(&self) -> Result<LocalFingerprints, kernel::Error> {
        Err(kernel::Error::Failed("the file vanished".to_owned()))
    }

    fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error> {
        Err(kernel::Error::Failed("the file vanished".to_owned()))
    }
}

#[test]
fn a_classification_error_fails_only_its_item() {
    let updated = RemoteTimestamp::Reported("2026-02-01".to_owned());
    let baseline_updated = RemoteTimestamp::Reported("2026-01-01".to_owned());
    let unreadable = UnreadableSinceWatermark;
    let absent = NeverCalled;
    let failing_key = ExternalId::new("ENG-1".to_owned());
    let neighbour_key = ExternalId::new("ENG-2".to_owned());
    let items = [
        PlanInput {
            id: "0001".to_owned(),
            external_id: Some(&failing_key),
            facts: RemoteFacts {
                presence: RemotePresence::Present,
                remote_updated: &updated,
            },
            dirty: work::sync::Dirtiness::Clean,
            baseline: BaselineEntry {
                remote_updated_at: &baseline_updated,
                fingerprints: agreeing_split(),
            },
            baseline_timestamp: 0,
            digests: &unreadable,
            comparison: &DifferingBodies,
        },
        PlanInput {
            id: "0002".to_owned(),
            external_id: Some(&neighbour_key),
            facts: RemoteFacts {
                presence: RemotePresence::Absent,
                remote_updated: &updated,
            },
            dirty: work::sync::Dirtiness::Clean,
            baseline: empty_baseline(),
            baseline_timestamp: 0,
            digests: &absent,
            comparison: &DifferingBodies,
        },
    ];

    let result =
        plan::plan(&items, SyncDirection::Bidirectional, &BTreeMap::new());

    assert_eq!(
        result.unclassifiable,
        vec![work::sync::Unclassifiable {
            id: "0001".to_owned(),
            reason: "the file vanished".to_owned(),
        }]
    );
    let planned: Vec<&str> = result
        .actions
        .iter()
        .map(work::sync::PlannedAction::id)
        .collect();
    assert_eq!(planned, vec!["0002"]);
}
