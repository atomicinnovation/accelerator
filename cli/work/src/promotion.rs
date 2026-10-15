//! Promoting a draft onto the issue a tracker creates for it: the durable
//! record of how far a promotion has got, and the pure decisions over it.
//!
//! The record carries everything needed to finish a promotion
//! deterministically, whoever finishes it, so a promotion interrupted at any
//! stage resumes without sending a second create.

use std::path::PathBuf;

use corpus::references::rewrite_references;
use corpus::references::IdShape;
use corpus::references::Renaming;
use tracker::ExternalId;
use tracker::RemoteTimestamp;

use crate::draft_id::DraftId;
use crate::retirement::RetirementFailure;
use crate::retirement::RetirementRefusal;
use crate::sync::classify;
use crate::sync::BaselineEntry;
use crate::sync::BaselineFingerprints;
use crate::sync::ItemComparison;
use crate::sync::ItemDigests;
use crate::sync::LegacyHashes;
use crate::sync::LocalFingerprints;
use crate::sync::PushOutcome;
use crate::sync::RemoteFingerprints;
use crate::sync::RemotePresence;
use crate::sync::RequestFingerprint;
use crate::sync::SplitFingerprints;
use crate::sync::Subject;
use crate::sync::SyncState;
use crate::tracker_key::TrackerKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionRecord {
    pub draft_id: DraftId,
    pub request: RequestFingerprint,
    pub content_digest: String,
    pub stage: PromotionStage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionStage {
    Attempted,
    Created {
        key: TrackerKey,
        created_remote_hash: Option<String>,
    },
    RemoteRetitled {
        key: TrackerKey,
        read_back: ReadBack,
    },
    RemoteKept {
        key: TrackerKey,
        reason: RemoteKeptReason,
    },
    Retiring {
        key: TrackerKey,
        baseline: Box<IntendedBaseline>,
        recovery_dir: PathBuf,
        before: Box<PromotionStage>,
    },
}

impl PromotionStage {
    /// The key the tracker gave the draft's issue, once one exists.
    #[must_use]
    pub const fn key(&self) -> Option<&TrackerKey> {
        match self {
            Self::Attempted => None,
            Self::Created { key, .. }
            | Self::RemoteRetitled { key, .. }
            | Self::RemoteKept { key, .. }
            | Self::Retiring { key, .. } => Some(key),
        }
    }
}

/// What a read of the remote issue saw: its body's digests, and the stamp
/// the tracker reported with it, so a baseline recorded from it proves the
/// issue unchanged without reading the body again.
///
/// `hash` is the legacy recipe's digest; `fingerprints` is absent for a
/// read-back recorded before the split recipe, which keeps the promotion
/// on the legacy recipe end to end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadBack {
    pub hash: String,
    pub fingerprints: Option<RemoteFingerprints>,
    pub updated: RemoteTimestamp,
}

/// A local file's digests under both recipes; `fingerprints` is absent for
/// one recorded before the split recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDigests {
    pub legacy: String,
    pub fingerprints: Option<LocalFingerprints>,
}

/// Why promotion left the remote issue's content as the tracker holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteKeptReason {
    Edited { read_back: ReadBack },
    UserNamedAdopt { read_back: ReadBack },
    UpdateFailed { read_back: ReadBack },
    NoHash { read_back: ReadBack },
}

impl RemoteKeptReason {
    #[must_use]
    pub const fn read_back(&self) -> &ReadBack {
        match self {
            Self::Edited { read_back }
            | Self::UserNamedAdopt { read_back }
            | Self::UpdateFailed { read_back }
            | Self::NoHash { read_back } => read_back,
        }
    }
}

/// The sync baseline a promotion records once its draft is retired, chosen
/// so the next sync does the right follow-up: nothing, a push of the key
/// H1, or a conflict dossier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntendedBaseline {
    pub remote_hash: RemoteHash,
    pub local: LocalDigests,
}

impl IntendedBaseline {
    /// The state the next sync finds the `promoted` item in once this
    /// baseline is recorded for it under `key`, as the classifier computes
    /// it, against a remote unchanged since it was read back.
    ///
    /// # Errors
    ///
    /// Propagates any [`ItemDigests`] read failure.
    pub fn next_sync(
        &self,
        key: &ExternalId,
        promoted: &dyn ItemDigests,
        comparison: &dyn ItemComparison,
    ) -> Result<SyncState, kernel::Error> {
        let stamp = match &self.remote_hash {
            RemoteHash::Known(read_back) => read_back.updated.clone(),
            RemoteHash::Unknown => RemoteTimestamp::NotRead,
        };
        let classification = classify(
            &Subject {
                external_id: Some(key),
                presence: RemotePresence::Present,
                remote_updated: &stamp,
                baseline_timestamp: 0,
            },
            &BaselineEntry {
                remote_updated_at: &stamp,
                fingerprints: self.fingerprints(),
            },
            promoted,
            comparison,
        )?;
        Ok(classification.state())
    }

    /// The fingerprints the next sync compares the promoted item against.
    #[must_use]
    pub fn fingerprints(&self) -> BaselineFingerprints {
        self.local.fingerprints.as_ref().map_or_else(
            || BaselineFingerprints::Legacy(self.legacy()),
            |local| {
                BaselineFingerprints::Split(SplitFingerprints {
                    local: Some(local.clone()),
                    remote: match &self.remote_hash {
                        RemoteHash::Known(read_back) => {
                            read_back.fingerprints.clone()
                        }
                        RemoteHash::Unknown => None,
                    },
                })
            },
        )
    }

    /// The legacy hashes recorded beside the fingerprints, so a binary that
    /// knows only the legacy recipe still reads the entry.
    #[must_use]
    pub fn legacy(&self) -> LegacyHashes {
        LegacyHashes {
            local: Some(self.local.legacy.clone()),
            remote: match &self.remote_hash {
                RemoteHash::Known(read_back) => Some(read_back.hash.clone()),
                RemoteHash::Unknown => None,
            },
        }
    }
}

/// `Unknown` is a remote hash the sync classifier always reads as changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteHash {
    Known(ReadBack),
    Unknown,
}

/// A promotion record as read from disk. `Unreadable` stays distinct from
/// `Absent`: a crash mid-write of a create's record must never read as "no
/// create was sent".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordState<'a> {
    Absent,
    Unreadable,
    Present(&'a PromotionRecord),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionMode {
    Standard,
    /// The user named the issue the draft's create reached.
    Adopt(TrackerKey),
    /// The user accepts that an earlier create may have reached the
    /// tracker, and asks for a fresh one.
    CreateAcceptingDuplicate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionStep {
    CreateIssue,
    VerifyThenAdopt(TrackerKey),
    RetitleRemote(TrackerKey),
    Retire(TrackerKey),
    Finish,
    Stop(NotPromoted),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotPromoted {
    TrackerUnreachable,
    ReadBackFailed(ExternalId),
    /// This run's create had no confirmed outcome, or the record is
    /// unreadable.
    CreateOutcomeUnknown,
    RequestRejected {
        detail: String,
    },
    /// An earlier run's create had no confirmed outcome.
    EarlierAttemptUnconfirmed,
    Refused(RetirementRefusal),
    RetirementFailed(RetirementFailure),
    /// The promotion record could not be written; `key` names the issue
    /// when one already exists.
    RecordUnwritable {
        key: Option<ExternalId>,
        detail: String,
    },
    AdoptedIssueMissing(ExternalId),
    AdoptConflictsWithRecordedKey {
        recorded: ExternalId,
    },
}

impl NotPromoted {
    /// The create outcome a draft left unpromoted for this reason reports.
    #[must_use]
    pub const fn create_outcome(&self) -> PushOutcome {
        match self {
            Self::TrackerUnreachable
            | Self::RecordUnwritable { key: None, .. } => {
                PushOutcome::LocalSave
            }
            Self::CreateOutcomeUnknown | Self::EarlierAttemptUnconfirmed => {
                PushOutcome::LoudTerminal
            }
            Self::RequestRejected { .. } => PushOutcome::Rejected,
            Self::Refused(RetirementRefusal::ItemNotFound(_))
            | Self::ReadBackFailed(_)
            | Self::RetirementFailed(RetirementFailure::RolledBack {
                ..
            })
            | Self::RecordUnwritable { key: Some(_), .. } => {
                PushOutcome::CreatedUnwritten
            }
            Self::Refused(_)
            | Self::AdoptedIssueMissing(_)
            | Self::AdoptConflictsWithRecordedKey { .. } => {
                PushOutcome::CreatedBlocked
            }
            Self::RetirementFailed(RetirementFailure::RestoreIncomplete {
                ..
            }) => PushOutcome::RetirementIncomplete,
        }
    }

    #[must_use]
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::TrackerUnreachable => "tracker-unreachable",
            Self::ReadBackFailed(_) => "read-back-failed",
            Self::CreateOutcomeUnknown => "remote-may-exist",
            Self::RequestRejected { .. } => "rejected",
            Self::EarlierAttemptUnconfirmed => "possible-duplicate",
            Self::Refused(refusal) => refusal.keyword(),
            Self::RetirementFailed(failure) => failure.keyword(),
            Self::RecordUnwritable { .. } => "record-unwritable",
            Self::AdoptedIssueMissing(_) => "adopted-issue-missing",
            Self::AdoptConflictsWithRecordedKey { .. } => {
                "adopt-conflicts-with-recorded-key"
            }
        }
    }

    /// The code `work sync` and `work promote` exit with for a draft left
    /// unpromoted, given whether an issue already exists for it: once one
    /// does, a draft that has vanished leaves it unlinked.
    #[must_use]
    pub const fn exit_code(&self, issue_exists: bool) -> u8 {
        match self {
            Self::TrackerUnreachable => RETRYABLE,
            Self::RequestRejected { .. } => REJECTED,
            Self::Refused(RetirementRefusal::ItemNotFound(_))
                if issue_exists =>
            {
                TERMINAL
            }
            Self::EarlierAttemptUnconfirmed
            | Self::Refused(_)
            | Self::AdoptedIssueMissing(_)
            | Self::AdoptConflictsWithRecordedKey { .. } => AWAITING_HUMAN,
            Self::CreateOutcomeUnknown
            | Self::ReadBackFailed(_)
            | Self::RetirementFailed(_)
            | Self::RecordUnwritable { key: Some(_), .. } => TERMINAL,
            Self::RecordUnwritable { key: None, .. } => INTERNAL_ERROR,
        }
    }
}

const INTERNAL_ERROR: u8 = 1;
const AWAITING_HUMAN: u8 = 4;
const RETRYABLE: u8 = 70;
const TERMINAL: u8 = 71;
const REJECTED: u8 = 75;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Promotion {
    /// Promoted onto the key; the promoted item is in the state the next
    /// sync will find it in.
    Completed(ExternalId, SyncState),
    AlreadyDone(ExternalId),
}

/// Whether promotion may rewrite the remote issue to carry the tracker key,
/// or must keep what the tracker holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Retitling {
    Retitle,
    Keep(RemoteKeptReason),
}

/// The placeholder a work item's ID is normalised back to, so two copies of
/// one content compare alike whatever ID each carries.
pub const ID_PLACEHOLDER: &str = "NNNN";

/// `text` with every whole-token occurrence of each of `ids`, in any case,
/// replaced by [`ID_PLACEHOLDER`].
#[must_use]
pub fn without_ids(text: &str, ids: &[&str]) -> String {
    ids.iter().fold(text.to_owned(), |normalised, id| {
        rewrite_references(
            &normalised,
            &Renaming {
                doc_type: "work-item",
                old_id: id,
                new_id: ID_PLACEHOLDER,
                shape: IdShape::Distinctive,
                path_spellings: &[],
            },
        )
    })
}

/// The step that resumes a promotion from its record.
///
/// The recovery modes replace a record that stands for no confirmed create;
/// once a key is recorded every mode resumes onto it, and an adopt naming a
/// different key stops.
#[must_use]
pub fn next_step(
    record: RecordState<'_>,
    mode: &PromotionMode,
) -> PromotionStep {
    let recorded = match record {
        RecordState::Present(record) => {
            record.stage.key().map(|key| (key, &record.stage))
        }
        RecordState::Absent | RecordState::Unreadable => None,
    };
    match (mode, recorded) {
        (PromotionMode::Adopt(named), None) => {
            PromotionStep::VerifyThenAdopt(named.clone())
        }
        (PromotionMode::CreateAcceptingDuplicate, None) => {
            PromotionStep::CreateIssue
        }
        (PromotionMode::Standard, None) => standard_start(record),
        (PromotionMode::Adopt(named), Some((key, _)))
            if !key.as_str().eq_ignore_ascii_case(named.as_str()) =>
        {
            PromotionStep::Stop(NotPromoted::AdoptConflictsWithRecordedKey {
                recorded: ExternalId::from(key),
            })
        }
        (_, Some((_, stage))) => resume(stage),
    }
}

const fn standard_start(record: RecordState<'_>) -> PromotionStep {
    match record {
        RecordState::Absent => PromotionStep::CreateIssue,
        RecordState::Unreadable => {
            PromotionStep::Stop(NotPromoted::CreateOutcomeUnknown)
        }
        RecordState::Present(_) => {
            PromotionStep::Stop(NotPromoted::EarlierAttemptUnconfirmed)
        }
    }
}

fn resume(stage: &PromotionStage) -> PromotionStep {
    match stage {
        PromotionStage::Attempted => {
            PromotionStep::Stop(NotPromoted::EarlierAttemptUnconfirmed)
        }
        PromotionStage::Created { key, .. } => {
            PromotionStep::RetitleRemote(key.clone())
        }
        PromotionStage::RemoteRetitled { key, .. }
        | PromotionStage::RemoteKept { key, .. } => {
            PromotionStep::Retire(key.clone())
        }
        PromotionStage::Retiring { .. } => PromotionStep::Finish,
    }
}

/// Whether the remote issue still holds only what promotion itself wrote.
///
/// `read_back_matches_draft` covers what a hash cannot: a record written
/// before the hash was known, and a promotion killed between its own H1
/// update and recording it, so the tool's own write is never mistaken for a
/// person's edit.
#[must_use]
pub fn remote_untouched(
    created_remote_hash: Option<&str>,
    read_back_hash: &str,
    read_back_matches_draft: bool,
) -> bool {
    created_remote_hash == Some(read_back_hash) || read_back_matches_draft
}

#[must_use]
pub fn retitling(
    created_remote_hash: Option<&str>,
    read_back: &ReadBack,
    read_back_matches_draft: bool,
) -> Retitling {
    if remote_untouched(
        created_remote_hash,
        &read_back.hash,
        read_back_matches_draft,
    ) {
        return Retitling::Retitle;
    }
    let read_back = read_back.clone();
    Retitling::Keep(if created_remote_hash.is_some() {
        RemoteKeptReason::Edited { read_back }
    } else {
        RemoteKeptReason::NoHash { read_back }
    })
}

/// The baseline a promotion settled at `disposition` records.
///
/// `None` for any stage but `RemoteRetitled` and `RemoteKept`: only a
/// promotion that has settled the remote side can know what the next sync
/// should do. A disposition whose read-back predates the split recipe
/// intends a legacy baseline.
#[must_use]
pub fn intended_baseline(
    disposition: &PromotionStage,
    draft: &LocalDigests,
    promoted: &LocalDigests,
    read_back_matches_promoted: bool,
) -> Option<IntendedBaseline> {
    let baseline = |remote_hash, read_back: &ReadBack, local: &LocalDigests| {
        IntendedBaseline {
            remote_hash,
            local: LocalDigests {
                legacy: local.legacy.clone(),
                fingerprints: local
                    .fingerprints
                    .clone()
                    .filter(|_| read_back.fingerprints.is_some()),
            },
        }
    };
    match disposition {
        PromotionStage::RemoteRetitled { read_back, .. } => Some(baseline(
            RemoteHash::Known(read_back.clone()),
            read_back,
            promoted,
        )),
        PromotionStage::RemoteKept {
            reason: reason @ RemoteKeptReason::UpdateFailed { .. },
            ..
        } => Some(baseline(
            RemoteHash::Known(reason.read_back().clone()),
            reason.read_back(),
            draft,
        )),
        PromotionStage::RemoteKept { reason, .. }
            if read_back_matches_promoted =>
        {
            Some(baseline(
                RemoteHash::Known(reason.read_back().clone()),
                reason.read_back(),
                draft,
            ))
        }
        PromotionStage::RemoteKept { reason, .. } => {
            Some(baseline(RemoteHash::Unknown, reason.read_back(), draft))
        }
        PromotionStage::Attempted
        | PromotionStage::Created { .. }
        | PromotionStage::Retiring { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tracker::ExternalId;

    use super::intended_baseline;
    use super::next_step;
    use super::remote_untouched;
    use super::retitling;
    use super::IntendedBaseline;
    use super::LocalDigests;
    use super::NotPromoted;
    use super::PromotionMode;
    use super::PromotionRecord;
    use super::PromotionStage;
    use super::PromotionStep;
    use super::ReadBack;
    use super::RecordState;
    use super::RemoteHash;
    use super::RemoteKeptReason;
    use super::Retitling;
    use crate::draft_id::DraftId;
    use crate::sync::BaselineFingerprints;
    use crate::sync::LegacyHashes;
    use crate::sync::LocalFingerprints;
    use crate::sync::PushOutcome;
    use crate::sync::RemoteFingerprints;
    use crate::sync::RequestFingerprint;
    use crate::sync::SplitFingerprints;
    use crate::sync::SyncState;
    use crate::tracker_key::TrackerKey;

    fn key() -> TrackerKey {
        TrackerKey::parse("PP-900")
            .unwrap_or_else(|| unreachable!("a tracker key"))
    }

    fn read_back(hash: &str) -> ReadBack {
        ReadBack {
            hash: hash.to_owned(),
            fingerprints: Some(remote_fingerprints(hash)),
            updated: tracker::RemoteTimestamp::Reported(format!("at-{hash}")),
        }
    }

    fn schema_two_read_back(hash: &str) -> ReadBack {
        ReadBack {
            fingerprints: None,
            ..read_back(hash)
        }
    }

    fn remote_fingerprints(hash: &str) -> RemoteFingerprints {
        RemoteFingerprints {
            body: format!("{hash}-body"),
            fields: format!("{hash}-fields"),
        }
    }

    fn local(digest: &str) -> LocalDigests {
        LocalDigests {
            legacy: digest.to_owned(),
            fingerprints: Some(LocalFingerprints {
                frontmatter: format!("{digest}-frontmatter"),
                fields: format!("{digest}-fields"),
                body: format!("{digest}-body"),
            }),
        }
    }

    fn legacy_local(digest: &str) -> LocalDigests {
        LocalDigests {
            fingerprints: None,
            ..local(digest)
        }
    }

    fn record(stage: PromotionStage) -> PromotionRecord {
        PromotionRecord {
            draft_id: DraftId::parse("draft-k7mq3x")
                .unwrap_or_else(|| unreachable!("a well-formed draft id")),
            request: RequestFingerprint {
                title: "Add search".to_owned(),
                digest: "request".to_owned(),
                attempted_at: 0,
                failure: None,
            },
            content_digest: "content".to_owned(),
            stage,
        }
    }

    fn kept(reason: RemoteKeptReason) -> PromotionStage {
        PromotionStage::RemoteKept { key: key(), reason }
    }

    fn retiring() -> PromotionStage {
        PromotionStage::Retiring {
            key: key(),
            baseline: Box::new(IntendedBaseline {
                remote_hash: RemoteHash::Unknown,
                local: local("draft"),
            }),
            recovery_dir: PathBuf::from("retirement-recovery/x"),
            before: Box::new(kept(RemoteKeptReason::NoHash {
                read_back: read_back("r"),
            })),
        }
    }

    #[test]
    fn an_unpromoted_draft_reports_the_create_outcome_its_reason_implies() {
        use crate::retirement::RetirementCause;
        use crate::retirement::RetirementCauseKind;
        use crate::retirement::RetirementFailure;
        use crate::retirement::RetirementRefusal;

        let key = ExternalId::new("PP-1".to_owned());
        let cause = RetirementCause {
            path: PathBuf::from("meta/work/a.md"),
            kind: RetirementCauseKind::ChangedSinceSnapshot,
        };
        let cases = [
            (NotPromoted::TrackerUnreachable, PushOutcome::LocalSave),
            (
                NotPromoted::RecordUnwritable {
                    key: None,
                    detail: String::new(),
                },
                PushOutcome::LocalSave,
            ),
            (NotPromoted::CreateOutcomeUnknown, PushOutcome::LoudTerminal),
            (
                NotPromoted::EarlierAttemptUnconfirmed,
                PushOutcome::LoudTerminal,
            ),
            (
                NotPromoted::RequestRejected {
                    detail: String::new(),
                },
                PushOutcome::Rejected,
            ),
            (
                NotPromoted::Refused(RetirementRefusal::ItemNotFound(
                    "draft-k7mq3x".to_owned(),
                )),
                PushOutcome::CreatedUnwritten,
            ),
            (
                NotPromoted::ReadBackFailed(key.clone()),
                PushOutcome::CreatedUnwritten,
            ),
            (
                NotPromoted::RetirementFailed(RetirementFailure::RolledBack {
                    cause: cause.clone(),
                }),
                PushOutcome::CreatedUnwritten,
            ),
            (
                NotPromoted::RecordUnwritable {
                    key: Some(key.clone()),
                    detail: String::new(),
                },
                PushOutcome::CreatedUnwritten,
            ),
            (
                NotPromoted::Refused(RetirementRefusal::TargetExists(
                    PathBuf::from("meta/work/PP-1-a.md"),
                )),
                PushOutcome::CreatedBlocked,
            ),
            (
                NotPromoted::AdoptedIssueMissing(key.clone()),
                PushOutcome::CreatedBlocked,
            ),
            (
                NotPromoted::AdoptConflictsWithRecordedKey { recorded: key },
                PushOutcome::CreatedBlocked,
            ),
            (
                NotPromoted::RetirementFailed(
                    RetirementFailure::RestoreIncomplete {
                        cause,
                        unrestored: Vec::new(),
                    },
                ),
                PushOutcome::RetirementIncomplete,
            ),
        ];

        for (reason, outcome) in cases {
            assert_eq!(reason.create_outcome(), outcome, "{reason:?}");
        }
    }

    #[test]
    fn every_whole_token_id_is_normalised_in_any_case() {
        assert_eq!(
            super::without_ids(
                "# PP-900: T\nsee draft-k7mq3x and DRAFT-K7MQ3X-notes\n",
                &["draft-k7mq3x", "PP-900"]
            ),
            "# NNNN: T\nsee NNNN and DRAFT-K7MQ3X-notes\n"
        );
    }

    #[test]
    fn next_step_under_the_standard_mode() {
        let edited = kept(RemoteKeptReason::Edited {
            read_back: read_back("r"),
        });
        let cases = [
            (None, PromotionStep::CreateIssue),
            (
                Some(PromotionStage::Attempted),
                PromotionStep::Stop(NotPromoted::EarlierAttemptUnconfirmed),
            ),
            (
                Some(PromotionStage::Created {
                    key: key(),
                    created_remote_hash: Some("h".to_owned()),
                }),
                PromotionStep::RetitleRemote(key()),
            ),
            (
                Some(PromotionStage::RemoteRetitled {
                    key: key(),
                    read_back: read_back("h"),
                }),
                PromotionStep::Retire(key()),
            ),
            (Some(edited), PromotionStep::Retire(key())),
            (Some(retiring()), PromotionStep::Finish),
        ];
        for (stage, expected) in cases {
            let held = stage.map(record);
            let state = held
                .as_ref()
                .map_or(RecordState::Absent, RecordState::Present);
            assert_eq!(
                next_step(state, &PromotionMode::Standard),
                expected,
                "{held:?}"
            );
        }
        assert_eq!(
            next_step(RecordState::Unreadable, &PromotionMode::Standard),
            PromotionStep::Stop(NotPromoted::CreateOutcomeUnknown)
        );
    }

    #[test]
    fn next_step_under_adopt_and_create_modes() {
        let other = TrackerKey::parse("PP-901")
            .unwrap_or_else(|| unreachable!("a tracker key"));
        let later_stages = [
            (
                PromotionStage::Created {
                    key: key(),
                    created_remote_hash: None,
                },
                PromotionStep::RetitleRemote(key()),
            ),
            (
                PromotionStage::RemoteRetitled {
                    key: key(),
                    read_back: read_back("h"),
                },
                PromotionStep::Retire(key()),
            ),
            (
                kept(RemoteKeptReason::UpdateFailed {
                    read_back: read_back("r"),
                }),
                PromotionStep::Retire(key()),
            ),
            (retiring(), PromotionStep::Finish),
        ];
        let adopt = PromotionMode::Adopt(key());
        let adopt_other = PromotionMode::Adopt(other);
        let create = PromotionMode::CreateAcceptingDuplicate;
        let attempted = record(PromotionStage::Attempted);
        for replaceable in [
            RecordState::Absent,
            RecordState::Unreadable,
            RecordState::Present(&attempted),
        ] {
            assert_eq!(
                next_step(replaceable, &adopt),
                PromotionStep::VerifyThenAdopt(key()),
                "{replaceable:?}"
            );
            assert_eq!(
                next_step(replaceable, &create),
                PromotionStep::CreateIssue,
                "{replaceable:?}"
            );
        }
        for (stage, resumed) in later_stages {
            let held = record(stage);
            let state = RecordState::Present(&held);
            assert_eq!(next_step(state, &adopt), resumed, "{held:?}");
            assert_eq!(next_step(state, &create), resumed, "{held:?}");
            assert_eq!(
                next_step(state, &adopt_other),
                PromotionStep::Stop(
                    NotPromoted::AdoptConflictsWithRecordedKey {
                        recorded: ExternalId::from(&key())
                    }
                ),
                "{held:?}"
            );
        }
    }

    #[test]
    fn adopting_the_recorded_key_in_another_case_resumes_onto_it() {
        let held = record(PromotionStage::RemoteRetitled {
            key: key(),
            read_back: read_back("h"),
        });
        let adopt_lowercase = PromotionMode::Adopt(
            TrackerKey::parse("pp-900")
                .unwrap_or_else(|| unreachable!("a tracker key")),
        );

        assert_eq!(
            next_step(RecordState::Present(&held), &adopt_lowercase),
            PromotionStep::Retire(key())
        );
    }

    /// A promoted item whose files digest under `digest`, beside a remote
    /// read back as `remote`, whose bodies compare as `equivalence`.
    struct Promoted {
        digest: &'static str,
        remote: Option<&'static str>,
        equivalence: crate::sync::BodyEquivalence,
    }

    impl crate::sync::ItemDigests for Promoted {
        fn mtime(&self) -> Result<Option<u64>, kernel::Error> {
            Ok(None)
        }

        fn legacy_local(&self) -> Result<String, kernel::Error> {
            Ok(self.digest.to_owned())
        }

        fn legacy_remote(&self) -> Result<Option<String>, kernel::Error> {
            Ok(self.remote.map(str::to_owned))
        }

        fn local(&self) -> Result<LocalFingerprints, kernel::Error> {
            Ok(local(self.digest)
                .fingerprints
                .unwrap_or_else(|| unreachable!("split fingerprints")))
        }

        fn remote(&self) -> Result<Option<RemoteFingerprints>, kernel::Error> {
            Ok(self.remote.map(remote_fingerprints))
        }
    }

    impl crate::sync::ItemComparison for Promoted {
        fn pull_residue(&self) -> Option<crate::sync::PullResidue> {
            None
        }

        fn body_equivalence(&self) -> crate::sync::BodyEquivalence {
            self.equivalence
        }

        fn fields_comparison(&self) -> crate::sync::FieldsComparison {
            crate::sync::FieldsComparison::Same
        }
    }

    #[test]
    fn the_next_sync_is_what_the_classifier_finds() -> Result<(), kernel::Error>
    {
        use crate::sync::BodyEquivalence;

        let external = tracker::ExternalId::new("PP-900".to_owned());
        let known = |digest: &str| IntendedBaseline {
            remote_hash: RemoteHash::Known(read_back("r")),
            local: local(digest),
        };
        let unknown = IntendedBaseline {
            remote_hash: RemoteHash::Unknown,
            local: local("promoted"),
        };
        let promoted = |equivalence| Promoted {
            digest: "promoted",
            remote: Some("r"),
            equivalence,
        };
        let next = |intended: &IntendedBaseline, promoted: &Promoted| {
            intended.next_sync(&external, promoted, promoted)
        };

        assert_eq!(
            next(&known("promoted"), &promoted(BodyEquivalence::Different))?,
            SyncState::Synced
        );
        assert_eq!(
            next(&known("draft"), &promoted(BodyEquivalence::Different))?,
            SyncState::LocallyModified
        );
        assert_eq!(
            next(&unknown, &promoted(BodyEquivalence::Different))?,
            SyncState::Conflict
        );
        assert_eq!(
            next(&unknown, &promoted(BodyEquivalence::Equivalent))?,
            SyncState::Synced,
            "an unknown remote hash over agreeing bodies is recovered"
        );
        Ok(())
    }

    #[test]
    fn a_fresh_intended_baseline_records_split_fingerprints() {
        let intended = IntendedBaseline {
            remote_hash: RemoteHash::Known(read_back("r")),
            local: local("promoted"),
        };

        assert_eq!(
            intended.fingerprints(),
            BaselineFingerprints::Split(SplitFingerprints {
                local: local("promoted").fingerprints,
                remote: Some(remote_fingerprints("r")),
            })
        );
        assert_eq!(
            intended.legacy(),
            LegacyHashes {
                local: Some("promoted".to_owned()),
                remote: Some("r".to_owned()),
            }
        );
    }

    #[test]
    fn an_unknown_remote_records_no_remote_fingerprints() {
        let intended = IntendedBaseline {
            remote_hash: RemoteHash::Unknown,
            local: local("draft"),
        };

        assert_eq!(
            intended.fingerprints(),
            BaselineFingerprints::Split(SplitFingerprints {
                local: local("draft").fingerprints,
                remote: None,
            })
        );
        assert_eq!(intended.legacy().remote, None);
    }

    #[test]
    fn every_new_reason_has_its_keyword() {
        assert_eq!(
            NotPromoted::AdoptedIssueMissing(ExternalId::from(&key()))
                .keyword(),
            "adopted-issue-missing"
        );
        assert_eq!(
            NotPromoted::AdoptConflictsWithRecordedKey {
                recorded: ExternalId::from(&key())
            }
            .keyword(),
            "adopt-conflicts-with-recorded-key"
        );
    }

    #[test]
    fn remote_untouched_by_hash_by_draft_match_and_neither() {
        assert!(remote_untouched(Some("h"), "h", false));
        assert!(remote_untouched(Some("h"), "edited", true));
        assert!(remote_untouched(None, "h", true));
        assert!(!remote_untouched(Some("h"), "edited", false));
        assert!(!remote_untouched(None, "h", false));
    }

    #[test]
    fn a_record_with_no_hash_and_no_draft_match_is_kept_as_no_hash() {
        assert_eq!(
            retitling(None, &read_back("r"), false),
            Retitling::Keep(RemoteKeptReason::NoHash {
                read_back: read_back("r")
            })
        );
        assert_eq!(
            retitling(Some("h"), &read_back("r"), false),
            Retitling::Keep(RemoteKeptReason::Edited {
                read_back: read_back("r")
            })
        );
        assert_eq!(retitling(None, &read_back("r"), true), Retitling::Retitle);
    }

    #[test]
    fn intended_baseline_for_every_disposition_and_reason() {
        let known = |hash: &str, digest: &str| {
            Some(IntendedBaseline {
                remote_hash: RemoteHash::Known(read_back(hash)),
                local: local(digest),
            })
        };
        let unknown = Some(IntendedBaseline {
            remote_hash: RemoteHash::Unknown,
            local: local("draft"),
        });
        let retitled = PromotionStage::RemoteRetitled {
            key: key(),
            read_back: read_back("after"),
        };
        let update_failed = kept(RemoteKeptReason::UpdateFailed {
            read_back: read_back("r"),
        });
        let untouched_but_unmatched = [
            kept(RemoteKeptReason::Edited {
                read_back: read_back("r"),
            }),
            kept(RemoteKeptReason::UserNamedAdopt {
                read_back: read_back("r"),
            }),
            kept(RemoteKeptReason::NoHash {
                read_back: read_back("r"),
            }),
        ];
        let (draft, promoted) = (local("draft"), local("promoted"));

        for matches in [false, true] {
            assert_eq!(
                intended_baseline(&retitled, &draft, &promoted, matches),
                known("after", "promoted"),
                "a retitled remote syncs as synced"
            );
            assert_eq!(
                intended_baseline(&update_failed, &draft, &promoted, matches),
                known("r", "draft"),
                "a failed H1 update is pushed by the next sync"
            );
        }
        for stage in &untouched_but_unmatched {
            assert_eq!(
                intended_baseline(stage, &draft, &promoted, true),
                known("r", "draft"),
                "differing only by the ID, the next sync pushes the key H1"
            );
            assert_eq!(
                intended_baseline(stage, &draft, &promoted, false),
                unknown,
                "any other difference raises a conflict"
            );
        }
        for stage in [
            PromotionStage::Attempted,
            PromotionStage::Created {
                key: key(),
                created_remote_hash: None,
            },
            retiring(),
        ] {
            assert_eq!(
                intended_baseline(&stage, &draft, &promoted, true),
                None,
                "{stage:?} has not settled the remote side"
            );
        }
        for stage in [&retitled, &update_failed, &untouched_but_unmatched[0]] {
            let fresh = intended_baseline(stage, &draft, &promoted, true)
                .map(|intended| intended.fingerprints());
            assert!(
                matches!(fresh, Some(BaselineFingerprints::Split(_))),
                "a fresh promotion records recipe 1: {fresh:?}"
            );
        }
    }

    #[test]
    fn a_schema_two_resumption_intends_a_legacy_baseline() {
        let resumed = [
            PromotionStage::RemoteRetitled {
                key: key(),
                read_back: schema_two_read_back("after"),
            },
            kept(RemoteKeptReason::UpdateFailed {
                read_back: schema_two_read_back("r"),
            }),
            kept(RemoteKeptReason::Edited {
                read_back: schema_two_read_back("r"),
            }),
        ];
        for stage in &resumed {
            for matches in [false, true] {
                let intended = intended_baseline(
                    stage,
                    &local("draft"),
                    &local("promoted"),
                    matches,
                )
                .unwrap_or_else(|| unreachable!("a settled remote side"));
                assert_eq!(
                    intended.local.fingerprints, None,
                    "{stage:?} keeps the legacy recipe"
                );
                assert!(
                    matches!(
                        intended.fingerprints(),
                        BaselineFingerprints::Legacy(_)
                    ),
                    "{stage:?}"
                );
            }
        }
        assert_eq!(
            intended_baseline(
                &resumed[0],
                &local("draft"),
                &local("promoted"),
                true
            )
            .map(|intended| intended.local),
            Some(legacy_local("promoted"))
        );
    }
}
