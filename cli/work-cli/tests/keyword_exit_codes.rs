//! Pins every push outcome's keyword and exit code, every keyword a batch
//! create reports, every reason a draft is not promoted, and every code a
//! re-key stops under, against a frozen expectation.
//!
//! The work skills branch on both the keyword `work create --push`,
//! `work create-batch`, `work sync` and `work promote` print and the code
//! each exits with, so the pairs are literals committed here rather than
//! derived from the code they guard.
#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use corpus::StoreError;
use tracker::ExternalId;
use work::create_batch::BatchKeyword;
use work::draft_id::DraftIdExhausted;
use work::identity::IdentityField;
use work::promotion::NotPromoted;
use work::rekey::entry::RekeyEntry;
use work::rekey::entry::RekeyTarget;
use work::rekey::fingerprint::Staleness;
use work::rekey::outcome::NotRekeyed;
use work::rekey::run_plan::Unreadability;
use work::rekey::run_plan::UnreadablePlan;
use work::rekey::write_set::Unreverted;
use work::retirement::RetirementCause;
use work::retirement::RetirementCauseKind;
use work::retirement::RetirementFailure;
use work::retirement::RetirementRefusal;
use work::sync::PushOutcome;

const FROZEN_KEYWORD_EXIT_CODES: &[(&str, u8)] = &[
    ("write-once", 0),
    ("retry", 70),
    ("local-save", 0),
    ("loud-terminal", 71),
    ("rejected", 75),
    ("created-unwritten", 71),
    ("created-blocked", 4),
    ("retirement-incomplete", 71),
];

const EVERY_OUTCOME: &[PushOutcome] = &[
    PushOutcome::WriteOnce,
    PushOutcome::Retry,
    PushOutcome::LocalSave,
    PushOutcome::LoudTerminal,
    PushOutcome::Rejected,
    PushOutcome::CreatedUnwritten,
    PushOutcome::CreatedBlocked,
    PushOutcome::RetirementIncomplete,
];

#[test]
fn every_keyword_maps_to_its_frozen_exit_code() {
    for outcome in EVERY_OUTCOME {
        let (_, frozen) = FROZEN_KEYWORD_EXIT_CODES
            .iter()
            .find(|(keyword, _)| *keyword == outcome.keyword())
            .unwrap_or_else(|| {
                panic!("no frozen code for keyword {}", outcome.keyword())
            });
        assert_eq!(
            outcome.exit_code(),
            *frozen,
            "{} has drifted from its contract",
            outcome.keyword()
        );
    }
    assert_eq!(EVERY_OUTCOME.len(), FROZEN_KEYWORD_EXIT_CODES.len());
}

const FROZEN_BATCH_KEYWORD_EXIT_CODES: &[(&str, u8)] = &[
    ("write-once", 0),
    ("retry", 70),
    ("local-save", 0),
    ("loud-terminal", 71),
    ("rejected", 75),
    ("created-unwritten", 71),
    ("created-blocked", 4),
    ("retirement-incomplete", 71),
    ("declined", 0),
    ("pending", 4),
];

#[test]
fn every_batch_keyword_maps_to_its_frozen_exit_code() {
    let every_keyword: Vec<BatchKeyword> = EVERY_OUTCOME
        .iter()
        .copied()
        .map(BatchKeyword::Pushed)
        .chain([BatchKeyword::Declined, BatchKeyword::Pending])
        .collect();
    for keyword in &every_keyword {
        let (_, frozen) = FROZEN_BATCH_KEYWORD_EXIT_CODES
            .iter()
            .find(|(frozen, _)| *frozen == keyword.keyword())
            .unwrap_or_else(|| {
                panic!("no frozen code for keyword {}", keyword.keyword())
            });
        assert_eq!(
            keyword.exit_code(),
            *frozen,
            "{} has drifted from its contract",
            keyword.keyword()
        );
    }
    assert_eq!(every_keyword.len(), FROZEN_BATCH_KEYWORD_EXIT_CODES.len());
}

/// `(keyword, exit before an issue exists, exit once one does)`.
const FROZEN_NOT_PROMOTED: &[(&str, u8, u8)] = &[
    ("tracker-unreachable", 70, 70),
    ("remote-may-exist", 71, 71),
    ("rejected", 75, 75),
    ("possible-duplicate", 4, 4),
    ("id-taken", 4, 4),
    ("key-linked", 4, 4),
    ("target-exists", 4, 4),
    ("item-not-found", 4, 71),
    ("adopted-issue-missing", 4, 4),
    ("adopt-conflicts-with-recorded-key", 4, 4),
    ("retirement-failed", 71, 71),
    ("read-back-failed", 71, 71),
    ("retirement-incomplete", 71, 71),
    ("record-unwritable", 1, 71),
];

fn key() -> ExternalId {
    ExternalId::new("PP-900".to_owned())
}

fn cause() -> RetirementCause {
    RetirementCause {
        path: PathBuf::from("meta/work/x.md"),
        kind: RetirementCauseKind::Store(StoreError::Io {
            path: "meta/work/x.md".to_owned(),
            detail: "disk full".to_owned(),
        }),
    }
}

/// One of each reason; a reason that carries its own key is given one only
/// when an issue exists.
fn every_reason(issue_exists: bool) -> Vec<NotPromoted> {
    let holder = PathBuf::from("meta/work/0001-holder.md");
    vec![
        NotPromoted::TrackerUnreachable,
        NotPromoted::CreateOutcomeUnknown,
        NotPromoted::RequestRejected {
            detail: String::new(),
        },
        NotPromoted::EarlierAttemptUnconfirmed,
        NotPromoted::Refused(RetirementRefusal::IdTaken {
            holder: holder.clone(),
            field: IdentityField::Id,
        }),
        NotPromoted::Refused(RetirementRefusal::KeyLinked { holder }),
        NotPromoted::Refused(RetirementRefusal::TargetExists(PathBuf::from(
            "meta/work/PP-900-x.md",
        ))),
        NotPromoted::Refused(RetirementRefusal::ItemNotFound(
            "draft-k7mq3x".to_owned(),
        )),
        NotPromoted::AdoptedIssueMissing(key()),
        NotPromoted::AdoptConflictsWithRecordedKey { recorded: key() },
        NotPromoted::RetirementFailed(RetirementFailure::RolledBack {
            cause: cause(),
        }),
        NotPromoted::ReadBackFailed(key()),
        NotPromoted::RetirementFailed(RetirementFailure::RestoreIncomplete {
            cause: cause(),
            unrestored: Vec::new(),
        }),
        NotPromoted::RecordUnwritable {
            key: issue_exists.then(key),
            detail: String::new(),
        },
    ]
}

#[test]
fn every_not_promoted_reason_maps_to_its_frozen_exit_code() {
    for issue_exists in [false, true] {
        let reasons = every_reason(issue_exists);
        assert_eq!(reasons.len(), FROZEN_NOT_PROMOTED.len());
        for reason in &reasons {
            let (_, before, after) = FROZEN_NOT_PROMOTED
                .iter()
                .find(|(keyword, ..)| *keyword == reason.keyword())
                .unwrap_or_else(|| {
                    panic!("no frozen code for keyword {}", reason.keyword())
                });
            let frozen = if issue_exists { after } else { before };
            assert_eq!(
                reason.exit_code(issue_exists),
                *frozen,
                "{} has drifted from its contract",
                reason.keyword()
            );
        }
    }
}

const FROZEN_NOT_REKEYED: &[(&str, u8)] = &[
    ("E_REKEY_IN_PROGRESS", 4),
    ("E_REKEY_PLAN_UNREADABLE", 4),
    ("E_REKEY_NOT_REVERTED", 4),
    ("E_REKEY_PARTLY_APPLIED", 4),
    ("E_REKEY_SYNC_UNFINISHED", 4),
    ("E_REKEY_PENDING_PUSH", 4),
    ("E_REKEY_ID_COLLISION", 4),
    ("E_REKEY_UNCLASSIFIABLE_ID", 4),
    ("E_REKEY_NUMBER_SPACE_EXHAUSTED", 4),
    ("E_REKEY_TARGET_EXISTS", 4),
    ("E_REKEY_PREVIEW_STALE", 4),
    ("E_REKEY_DIRTY_WORKING_COPY", 4),
    ("E_REKEY_WORKING_COPY_UNPROBEABLE", 4),
    ("E_REKEY_ENTRY_REFUSED", 4),
    ("E_REKEY_ENTRY_ROLLED_BACK", 4),
    ("E_REKEY_RESTORE_INCOMPLETE", 4),
    ("E_REKEY_STATE_UNWRITABLE", 1),
    ("E_IDENTITY_RUN_BUSY", 1),
    ("E_REKEY_TRACKER_UNREACHABLE", 70),
    ("E_DRAFT_ID_EXHAUSTED", 1),
];

fn rekey_entry() -> RekeyEntry {
    RekeyEntry {
        old: "0042".to_owned(),
        target: RekeyTarget::Renumbered("ACC-0042".to_owned()),
    }
}

fn every_not_rekeyed() -> Vec<NotRekeyed> {
    let item = PathBuf::from("meta/work/0042-x.md");
    vec![
        NotRekeyed::InProgress(None),
        NotRekeyed::PlanUnreadable(UnreadablePlan {
            path: PathBuf::from(".accelerator/state/rekey/run-plan.json"),
            cause: Unreadability::NewerSchema(2),
        }),
        NotRekeyed::NotReverted(Unreverted::default()),
        NotRekeyed::PartlyApplied {
            items: Vec::new(),
            recoveries: Vec::new(),
        },
        NotRekeyed::SyncUnfinished(Vec::new()),
        NotRekeyed::PendingPush(Vec::new()),
        NotRekeyed::Collision {
            first: item.clone(),
            second: item.clone(),
            target: "ACC-0042".to_owned(),
        },
        NotRekeyed::Unclassifiable(item.clone()),
        NotRekeyed::FreshNumbersExhausted {
            needed: 2,
            remaining: 1,
            pattern: String::new(),
        },
        NotRekeyed::KeptNumberAboveCap {
            item: item.clone(),
            number: 12_345,
            pattern: String::new(),
        },
        NotRekeyed::TargetExists(item.clone()),
        NotRekeyed::PreviewStale(Staleness::SchemeChanged),
        NotRekeyed::DirtyWorkingCopy(Vec::new()),
        NotRekeyed::WorkingCopyUnprobeable(String::new()),
        NotRekeyed::EntryRefused {
            entry: rekey_entry(),
            refusal: RetirementRefusal::TargetExists(item.clone()),
        },
        NotRekeyed::EntryRolledBack {
            entry: rekey_entry(),
            cause: Box::new(cause()),
        },
        NotRekeyed::RestoreIncomplete {
            entry: rekey_entry(),
            recovery: item,
        },
        NotRekeyed::StateUnwritable(String::new()),
        NotRekeyed::IdentityRunBusy(None),
        NotRekeyed::TrackerUnreachable(String::new()),
        NotRekeyed::DraftIdExhausted(DraftIdExhausted { attempts: 16 }),
    ]
}

#[test]
fn every_rekey_outcome_maps_to_its_frozen_code() {
    let outcomes = every_not_rekeyed();
    for outcome in &outcomes {
        let (_, frozen) = FROZEN_NOT_REKEYED
            .iter()
            .find(|(code, _)| *code == outcome.code())
            .unwrap_or_else(|| {
                panic!("no frozen exit code for {}", outcome.code())
            });
        assert_eq!(
            outcome.exit_code(),
            *frozen,
            "{} has drifted from its contract",
            outcome.code()
        );
    }
    let codes: BTreeSet<&str> = outcomes.iter().map(NotRekeyed::code).collect();
    assert_eq!(codes.len(), FROZEN_NOT_REKEYED.len());
}
