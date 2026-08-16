//! The planner's branch table, at unit level with no fake tracker needed:
//! `needs_body_read`'s partition, the `RemotePresence` mapping, resolution
//! application over `Prompt`, and `Noop` suppression.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;

use tracker::ExternalId;
use tracker::RemoteTimestamp;
use work::sync::plan;
use work::sync::BaselineEntry;
use work::sync::ItemDigests;
use work::sync::PlanInput;
use work::sync::RemoteFacts;
use work::sync::RemotePresence;
use work::sync::Resolution;
use work::sync::SyncDirection;
use work::sync::SyncState;

struct NeverCalled;

impl ItemDigests for NeverCalled {
    fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
        panic!("mtime should not be called for a presence-absent item")
    }

    fn local(&self) -> Result<String, kernel::Error> {
        panic!("local should not be called for a presence-absent item")
    }

    fn remote_body(&self) -> Result<Option<String>, kernel::Error> {
        panic!("remote_body should not be called for a presence-absent item")
    }
}

const fn empty_baseline() -> BaselineEntry<'static> {
    BaselineEntry {
        remote_updated_at: &RemoteTimestamp::NotReported,
        remote_hash: None,
        local_hash: None,
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
        remote_hash: Some("h"),
        local_hash: Some("h"),
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
            remote_hash: Some("h"),
            local_hash: Some("h"),
        },
        baseline_timestamp: 0,
        digests: &digests,
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

        fn local(&self) -> Result<String, kernel::Error> {
            Ok(self.0.to_owned())
        }

        fn remote_body(&self) -> Result<Option<String>, kernel::Error> {
            Ok(Some(self.0.to_owned()))
        }
    }
    Agreeing(hash)
}

#[test]
fn an_absent_item_plans_as_remote_absent_and_noop() -> Result<(), kernel::Error>
{
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
    };

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &BTreeMap::new())?;

    assert_eq!(result.actions.len(), 1);
    assert_eq!(result.actions[0].state, SyncState::RemoteAbsent);
    assert_eq!(result.actions[0].action, work::sync::Action::Noop);
    Ok(())
}

#[test]
fn an_indeterminate_item_plans_as_indeterminate_and_noop(
) -> Result<(), kernel::Error> {
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
    };

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &BTreeMap::new())?;

    assert_eq!(result.actions.len(), 1);
    assert_eq!(result.actions[0].state, SyncState::Indeterminate);
    assert_eq!(result.actions[0].action, work::sync::Action::Noop);
    Ok(())
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
            remote_hash: Some("stale"),
            local_hash: Some("stale"),
        },
        baseline_timestamp: 0,
        digests,
    }
}

#[test]
fn a_resolution_of_accept_remote_turns_a_prompt_into_a_pull(
) -> Result<(), kernel::Error> {
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
        plan::plan(&[item], SyncDirection::Bidirectional, &resolutions)?;

    assert_eq!(result.actions[0].state, SyncState::Conflict);
    assert_eq!(result.actions[0].action, work::sync::Action::Pull);
    Ok(())
}

#[test]
fn a_resolution_of_push_local_turns_a_prompt_into_a_push(
) -> Result<(), kernel::Error> {
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
    let mut resolutions = BTreeMap::new();
    resolutions.insert("0007".to_owned(), Resolution::PushLocal);

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &resolutions)?;

    assert_eq!(result.actions[0].action, work::sync::Action::Push);
    Ok(())
}

#[test]
fn a_resolution_of_skip_leaves_the_prompt_unresolved(
) -> Result<(), kernel::Error> {
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
        plan::plan(&[item], SyncDirection::Bidirectional, &resolutions)?;

    assert_eq!(result.actions[0].action, work::sync::Action::Prompt);
    Ok(())
}

#[test]
fn a_stale_resolution_naming_an_id_that_did_not_prompt_is_inert(
) -> Result<(), kernel::Error> {
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
            remote_hash: Some("h"),
            local_hash: Some("h"),
        },
        baseline_timestamp: 0,
        digests: &digests,
    };
    let mut resolutions = BTreeMap::new();
    resolutions.insert("0009".to_owned(), Resolution::AcceptRemote);

    let result =
        plan::plan(&[item], SyncDirection::Bidirectional, &resolutions)?;

    assert_eq!(result.actions[0].state, SyncState::Synced);
    assert_eq!(result.actions[0].action, work::sync::Action::Noop);
    Ok(())
}

#[test]
fn pull_and_push_counts_reflect_only_their_own_action(
) -> Result<(), kernel::Error> {
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
            remote_hash: Some("h"),
            local_hash: Some("h"),
        },
        baseline_timestamp: 0,
        digests: &digests,
    };

    let result = plan::plan(
        &[synced_item],
        SyncDirection::Bidirectional,
        &BTreeMap::new(),
    )?;

    assert_eq!(result.pull_count(), 0);
    assert_eq!(result.push_count(), 0);
    Ok(())
}
