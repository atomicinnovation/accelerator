//! Pins every push outcome's keyword and exit code, every keyword a batch
//! create reports, and every reason a draft is not promoted, against a
//! frozen expectation.
//!
//! The work skills branch on both the keyword `work create --push`,
//! `work create-batch`, `work sync` and `work promote` print and the code
//! each exits with, so the pairs are literals committed here rather than
//! derived from the code they guard.
#![allow(clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use corpus::StoreError;
use tracker::ExternalId;
use work::create_batch::BatchKeyword;
use work::identity::IdentityField;
use work::promotion::NotPromoted;
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
