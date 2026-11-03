//! Which conflicts a run settled, and so whose dossiers it may retire.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use tracker::ExternalId;
use tracker::RemoteTimestamp;
use tracker::TrackerError;
use work::inventory::CorpusInventory;
use work::inventory::MalformedCause;
use work::inventory::Unidentified;
use work::inventory::UnidentifiedReason;
use work::sync::Action;
use work::sync::BaselineEntry;
use work::sync::BaselineFingerprints;
use work::sync::BodyEquivalence;
use work::sync::Dirtiness;
use work::sync::FieldDelivery;
use work::sync::FieldsComparison;
use work::sync::ItemComparison;
use work::sync::ItemDigests;
use work::sync::LegacyHashes;
use work::sync::LocalFingerprints;
use work::sync::PendingPushOrder;
use work::sync::PlanInput;
use work::sync::PlanOrders;
use work::sync::PlannedAction;
use work::sync::PullResidue;
use work::sync::RemoteFacts;
use work::sync::RemoteFingerprints;
use work::sync::RemotePresence;
use work::sync::Resolution;
use work::sync::SyncDirection;
use work::sync::SyncState;
use work::sync::Unclassifiable;
use work_adapters::sync::apply::ApplyError;
use work_adapters::sync::apply::ItemProgress;
use work_adapters::sync::baseline::Degradation;
use work_adapters::sync::dossiers::dossiers_gone_stale;
use work_adapters::sync::dossiers::dossiers_to_refresh;
use work_adapters::sync::dossiers::ConflictStatus;
use work_adapters::sync::dossiers::DossierAuthority;
use work_adapters::sync::operation::SyncOperation;
use work_adapters::sync::report::CorpusScope;
use work_adapters::sync::report::Finalisation;
use work_adapters::sync::run::DiscoveryStatus;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::ReportedItem;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;

const ID: &str = "0001";

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
        Ok(LocalFingerprints {
            frontmatter: self.0.to_owned(),
            fields: self.0.to_owned(),
            body: self.0.to_owned(),
        })
    }

    fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error> {
        Ok(Some(RemoteFingerprints {
            body: self.0.to_owned(),
            fields: self.0.to_owned(),
        }))
    }
}

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

/// How the planner sees an item both sides changed, or one changed
/// remotely beside an uncommitted local edit.
#[derive(Clone, Copy)]
enum Prompted {
    Conflict,
    DirtyRemoteChange,
}

fn planned_for(
    id: &str,
    prompted: Prompted,
    orders: &PlanOrders<'_>,
) -> PlannedAction {
    let key = ExternalId::new("ENG-1".to_owned());
    let baseline_updated = RemoteTimestamp::Reported("old".to_owned());
    let updated = RemoteTimestamp::Reported("new".to_owned());
    let (local_hash, dirty) = match prompted {
        Prompted::Conflict => ("stale", Dirtiness::Clean),
        Prompted::DirtyRemoteChange => ("h", Dirtiness::Dirty),
    };
    let digests = Agreeing("h");
    let input = PlanInput {
        id: id.to_owned(),
        external_id: Some(&key),
        facts: RemoteFacts {
            presence: RemotePresence::Present,
            remote_updated: &updated,
        },
        dirty,
        baseline: BaselineEntry {
            remote_updated_at: &baseline_updated,
            fingerprints: BaselineFingerprints::Legacy(LegacyHashes {
                local: Some(local_hash.to_owned()),
                remote: Some("stale".to_owned()),
            }),
        },
        baseline_timestamp: 0,
        digests: &digests,
        comparison: &DifferingBodies,
    };
    let mut plan =
        work::sync::plan(&[input], SyncDirection::Bidirectional, orders);
    plan.actions.remove(0)
}

fn resolved_as(
    id: &str,
    prompted: Prompted,
    resolution: Resolution,
) -> PlannedAction {
    let resolutions = BTreeMap::from([(id.to_owned(), resolution)]);
    let planned =
        planned_for(id, prompted, &PlanOrders::resolving(&resolutions));
    assert!(planned.resolution().is_some(), "the planner resolved {id}");
    planned
}

fn resolved(prompted: Prompted, resolution: Resolution) -> PlannedAction {
    resolved_as(ID, prompted, resolution)
}

fn prompt(id: &str) -> PlannedAction {
    planned_for(
        id,
        Prompted::Conflict,
        &PlanOrders::resolving(&BTreeMap::new()),
    )
}

fn relinked() -> PlannedAction {
    let pending_push =
        BTreeMap::from([(ID.to_owned(), PendingPushOrder::Create)]);
    let unconfirmed_links = BTreeSet::from([ID.to_owned()]);
    let planned = planned_for(
        ID,
        Prompted::Conflict,
        &PlanOrders {
            resolutions: &BTreeMap::new(),
            pending_push: &pending_push,
            unconfirmed_links: &unconfirmed_links,
        },
    );
    assert_eq!(planned.action(), Action::Relink);
    planned
}

fn planned(state: SyncState, action: Action) -> PlannedAction {
    PlannedAction::new(ID, state, action)
}

const fn reported(
    planned: PlannedAction,
    outcome: ItemOutcome,
) -> ReportedItem {
    ReportedItem {
        planned,
        outcome,
        validation: None,
        kept_edits: None,
        leftover_marker: None,
        notice: None,
    }
}

fn untouched_failure() -> ItemOutcome {
    ItemOutcome::Failed(ApplyError::Io {
        item_id: ID.to_owned(),
        operation: SyncOperation::WriteLocal,
        progress: ItemProgress::NothingLanded,
        detail: "Permission denied".to_owned(),
    })
}

fn partial_failure() -> ItemOutcome {
    ItemOutcome::Failed(ApplyError::Io {
        item_id: ID.to_owned(),
        operation: SyncOperation::BaselineSet,
        progress: ItemProgress::MayHaveLanded,
        detail: "disk full".to_owned(),
    })
}

fn terminal_tracker_failure() -> ItemOutcome {
    ItemOutcome::Failed(ApplyError::Tracker {
        item_id: ID.to_owned(),
        operation: SyncOperation::Update,
        source: TrackerError::Terminal {
            detail: "timed out after sending".to_owned(),
        },
    })
}

fn status(planned: PlannedAction, outcome: ItemOutcome) -> ConflictStatus {
    ConflictStatus::of(&reported(planned, outcome))
}

fn every_resolution() -> Vec<PlannedAction> {
    vec![
        resolved(Prompted::Conflict, Resolution::AcceptRemote),
        resolved(Prompted::Conflict, Resolution::PushLocal),
        resolved(Prompted::DirtyRemoteChange, Resolution::AcceptRemote),
        resolved(Prompted::DirtyRemoteChange, Resolution::PushLocal),
        relinked(),
    ]
}

#[test]
fn a_prompted_item_is_awaiting_resolution() {
    for prompted in [Prompted::Conflict, Prompted::DirtyRemoteChange] {
        let prompt =
            planned_for(ID, prompted, &PlanOrders::resolving(&BTreeMap::new()));
        assert_eq!(prompt.action(), Action::Prompt);

        assert_eq!(
            status(prompt, ItemOutcome::NotApplied),
            ConflictStatus::AwaitingResolution
        );
    }
}

#[test]
fn a_skipped_resolution_is_awaiting_resolution() {
    let skipped = planned_for(
        ID,
        Prompted::Conflict,
        &PlanOrders::resolving(&BTreeMap::from([(
            ID.to_owned(),
            Resolution::Skip,
        )])),
    );

    assert_eq!(
        status(skipped, ItemOutcome::NotApplied),
        ConflictStatus::AwaitingResolution
    );
}

#[test]
fn a_resolution_is_pending_failed_or_settled_by_its_outcome() {
    for resolution in every_resolution() {
        assert_eq!(
            status(resolution.clone(), ItemOutcome::NotApplied),
            ConflictStatus::ResolutionPending,
            "{resolution:?} previewed"
        );
        for failure in [
            untouched_failure(),
            partial_failure(),
            terminal_tracker_failure(),
        ] {
            assert_eq!(
                status(resolution.clone(), failure),
                ConflictStatus::ResolutionFailed,
                "{resolution:?} failed"
            );
        }
        assert_eq!(
            status(resolution.clone(), ItemOutcome::Applied),
            ConflictStatus::Settled,
            "{resolution:?} applied"
        );
    }
}

#[test]
fn a_recorded_baseline_takes_the_item_out_of_conflict_only_once_applied() {
    for state in [SyncState::Synced, SyncState::LocallyModified] {
        let record = || planned(state, Action::RecordBaseline);

        assert_eq!(
            status(record(), ItemOutcome::Applied),
            ConflictStatus::NotInConflict
        );
        assert_eq!(
            status(record(), ItemOutcome::NotApplied),
            ConflictStatus::Unobserved
        );
        assert_eq!(
            status(record(), untouched_failure()),
            ConflictStatus::Unobserved
        );
    }
}

#[test]
fn an_item_the_run_could_not_judge_is_unobserved() {
    let unjudged = [
        planned(SyncState::Conflict, Action::SkipConflict),
        planned(SyncState::RemotelyModified, Action::SkipDirty),
        planned(SyncState::Indeterminate, Action::Noop),
        planned(SyncState::RemoteAbsent, Action::Noop),
        planned(SyncState::RemotelyModified, Action::Noop),
        planned(SyncState::LocallyModified, Action::Noop),
    ];
    for planned in unjudged {
        for outcome in [
            || ItemOutcome::NotApplied,
            || ItemOutcome::Applied,
            untouched_failure,
        ] {
            assert_eq!(
                status(planned.clone(), outcome()),
                ConflictStatus::Unobserved,
                "{planned:?}"
            );
        }
    }
}

#[test]
fn an_item_whose_digest_recipe_is_unknown_is_unobserved() {
    assert_eq!(
        status(
            planned(SyncState::Synced, Action::Noop),
            ItemOutcome::UnsupportedRecipe
        ),
        ConflictStatus::Unobserved
    );
}

#[test]
fn an_item_no_conflict_touched_is_not_in_conflict() {
    let untroubled = [
        planned(SyncState::RemotelyModified, Action::Pull),
        planned(
            SyncState::LocallyModified,
            Action::Push(FieldDelivery::Send),
        ),
        planned(SyncState::LocallyModified, Action::Retitle),
        planned(SyncState::Synced, Action::Noop),
        planned(SyncState::Unsynced, Action::Noop),
        planned(SyncState::Unsynced, Action::CreateFromRemote),
        planned(SyncState::Unsynced, Action::CreateFromLocal),
    ];
    for planned in untroubled {
        for outcome in [|| ItemOutcome::Applied, untouched_failure] {
            assert_eq!(
                status(planned.clone(), outcome()),
                ConflictStatus::NotInConflict,
                "{planned:?}"
            );
        }
    }
}

#[test]
fn a_combination_the_planner_cannot_produce_is_unobserved() {
    let impossible = [
        planned(SyncState::Synced, Action::Prompt),
        planned(SyncState::Synced, Action::Pull),
        planned(SyncState::Conflict, Action::Overwrite),
        planned(SyncState::Unsynced, Action::RecordBaseline),
    ];
    for planned in impossible {
        assert_eq!(
            status(planned.clone(), ItemOutcome::Applied),
            ConflictStatus::Unobserved,
            "{planned:?}"
        );
    }
}

const fn report_of(
    scope: CorpusScope,
    mode: RunMode,
    inventory: CorpusInventory,
    reported: Vec<ReportedItem>,
) -> RunReport {
    RunReport {
        scope,
        mode,
        inventory,
        identity: Vec::new(),
        promotions: Vec::new(),
        deferred: 0,
        reported,
        unclassifiable: Vec::new(),
        refused_orders: Vec::new(),
        adoptions: Vec::new(),
        unretired_adoptions: Vec::new(),
        read_failure: None,
        baseline_degradation: Degradation::None,
        finalisation: Finalisation::Finalised,
        dossiers: Vec::new(),
        stale_dossier_ids: Vec::new(),
        discovery: DiscoveryStatus::Ran { found: 0 },
        discovery_deferral: None,
        keyed_read_budget_limited: false,
    }
}

fn item(id: &str, state: SyncState, action: Action) -> ReportedItem {
    reported(PlannedAction::new(id, state, action), ItemOutcome::Applied)
}

fn mixed_run() -> Vec<ReportedItem> {
    vec![
        item("0001", SyncState::RemotelyModified, Action::Pull),
        reported(
            PlannedAction::new("0002", SyncState::Conflict, Action::Prompt),
            ItemOutcome::NotApplied,
        ),
        item("0003", SyncState::Indeterminate, Action::Noop),
    ]
}

fn ids(ids: &[&str]) -> CorpusInventory {
    CorpusInventory::complete(ids.iter().map(|id| (*id).to_owned()))
}

#[test]
fn a_whole_corpus_run_retires_settled_and_orphaned_dossiers() {
    let report = report_of(
        CorpusScope::WholeCorpus,
        RunMode::Apply,
        ids(&["0001", "0002", "0003", "0004"]),
        mixed_run(),
    );
    let authority = DossierAuthority::for_run(&report);

    assert!(authority.may_retire("0001"), "settled");
    assert!(authority.may_retire("0009"), "orphaned");
    assert!(!authority.may_retire("0002"), "awaiting resolution");
    assert!(!authority.may_retire("0003"), "unobserved");
    assert!(!authority.may_retire("0004"), "deferred corpus item");
    assert!(authority.retires_artefacts());
}

#[test]
fn an_incomplete_inventory_keeps_orphans() {
    let mut inventory = ids(&["0001"]);
    inventory.unidentified.push(Unidentified {
        path: "meta/work/0005-broken.md".into(),
        reason: UnidentifiedReason::Malformed(MalformedCause::UnclosedFence),
    });
    let report = report_of(
        CorpusScope::WholeCorpus,
        RunMode::Apply,
        inventory,
        mixed_run(),
    );

    assert!(!DossierAuthority::for_run(&report).may_retire("0009"));
}

#[test]
fn an_empty_inventory_keeps_orphans() {
    let report = report_of(
        CorpusScope::WholeCorpus,
        RunMode::Apply,
        CorpusInventory::default(),
        Vec::new(),
    );

    assert!(!DossierAuthority::for_run(&report).may_retire("0009"));
}

#[test]
fn a_targeted_run_retires_only_reported_settled_dossiers() {
    let report = report_of(
        CorpusScope::Targeted,
        RunMode::Apply,
        ids(&["0001", "0002", "0003", "0004"]),
        mixed_run(),
    );
    let authority = DossierAuthority::for_run(&report);

    assert!(authority.may_retire("0001"));
    assert!(!authority.may_retire("0002"));
    assert!(!authority.may_retire("0003"));
    assert!(!authority.may_retire("0004"));
    assert!(!authority.may_retire("0009"));
}

#[test]
fn a_preview_retires_nothing() {
    let report = report_of(
        CorpusScope::WholeCorpus,
        RunMode::Preview,
        ids(&["0001"]),
        mixed_run(),
    );
    let authority = DossierAuthority::for_run(&report);

    assert!(!authority.may_retire("0001"));
    assert!(!authority.may_retire("0009"));
    assert!(!authority.retires_artefacts());
}

#[test]
fn an_unclassifiable_conflicted_item_keeps_its_dossier() {
    for scope in [CorpusScope::Targeted, CorpusScope::WholeCorpus] {
        let mut report = report_of(
            scope,
            RunMode::Apply,
            ids(&["0001", "0007"]),
            mixed_run(),
        );
        report.unclassifiable.push(Unclassifiable {
            id: "0007".to_owned(),
            reason: "the local file could not be hashed".to_owned(),
        });

        assert!(!DossierAuthority::for_run(&report).may_retire("0007"));
    }
}

fn resolution_of(id: &str, outcome: ItemOutcome) -> ReportedItem {
    reported(
        resolved_as(id, Prompted::Conflict, Resolution::AcceptRemote),
        outcome,
    )
}

fn resolutions_in_every_state() -> Vec<ReportedItem> {
    vec![
        reported(prompt("0001"), ItemOutcome::NotApplied),
        resolution_of("0002", ItemOutcome::NotApplied),
        resolution_of("0003", untouched_failure()),
        resolution_of("0004", partial_failure()),
        resolution_of("0005", ItemOutcome::Applied),
        reported(
            resolved_as("0006", Prompted::Conflict, Resolution::PushLocal),
            terminal_tracker_failure(),
        ),
        item("0007", SyncState::Conflict, Action::SkipConflict),
    ]
}

#[test]
fn an_awaiting_pending_or_untouched_resolution_gets_a_fresh_dossier() {
    let items = resolutions_in_every_state();

    assert_eq!(
        dossiers_to_refresh(&items),
        BTreeSet::from(["0001", "0002", "0003"])
    );
}

#[test]
fn a_partially_applied_resolution_failure_keeps_its_prior_dossier() {
    let items = resolutions_in_every_state();

    assert!(!dossiers_to_refresh(&items).contains("0004"));
    assert!(dossiers_gone_stale(&items).contains("0004"));
}

#[test]
fn a_terminal_resolution_overwrite_failure_keeps_its_prior_dossier() {
    let items = resolutions_in_every_state();

    assert!(!dossiers_to_refresh(&items).contains("0006"));
    assert_eq!(
        dossiers_gone_stale(&items),
        BTreeSet::from(["0004", "0006"])
    );
}

fn awaiting_human(items: Vec<ReportedItem>) -> BTreeSet<String> {
    let report = report_of(
        CorpusScope::WholeCorpus,
        RunMode::Apply,
        CorpusInventory::default(),
        items,
    );
    report
        .awaiting_human()
        .map(|item| item.planned.id().to_owned())
        .collect()
}

#[test]
fn every_awaiting_resolution_item_is_awaiting_human() {
    let items = resolutions_in_every_state();
    let awaiting: BTreeSet<String> = items
        .iter()
        .filter(|item| {
            ConflictStatus::of(item) == ConflictStatus::AwaitingResolution
        })
        .map(|item| item.planned.id().to_owned())
        .collect();

    assert!(awaiting.is_subset(&awaiting_human(items)));
    assert!(!awaiting.is_empty());
}

#[test]
fn no_resolution_pending_or_failed_item_is_awaiting_human() {
    let human = awaiting_human(resolutions_in_every_state());

    for id in ["0002", "0003", "0004", "0006"] {
        assert!(!human.contains(id), "{id}");
    }
}
