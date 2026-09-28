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
use crate::sync::RequestFingerprint;

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
        key: ExternalId,
        created_remote_hash: Option<String>,
    },
    RemoteRetitled {
        key: ExternalId,
        read_back: ReadBack,
    },
    RemoteKept {
        key: ExternalId,
        reason: RemoteKeptReason,
    },
    Retiring {
        key: ExternalId,
        baseline: IntendedBaseline,
        recovery_dir: PathBuf,
        before: Box<PromotionStage>,
    },
}

impl PromotionStage {
    /// The key the tracker gave the draft's issue, once one exists.
    #[must_use]
    pub const fn key(&self) -> Option<&ExternalId> {
        match self {
            Self::Attempted => None,
            Self::Created { key, .. }
            | Self::RemoteRetitled { key, .. }
            | Self::RemoteKept { key, .. }
            | Self::Retiring { key, .. } => Some(key),
        }
    }
}

/// What a read of the remote issue saw: its body's digest, and the stamp
/// the tracker reported with it, so a baseline recorded from it proves the
/// issue unchanged without reading the body again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadBack {
    pub hash: String,
    pub updated: RemoteTimestamp,
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
    pub local_hash: String,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionStep {
    CreateIssue,
    RetitleRemote(ExternalId),
    Retire(ExternalId),
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
}

impl NotPromoted {
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Promotion {
    Completed(ExternalId),
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
#[must_use]
pub fn next_step(
    record: RecordState<'_>,
    mode: &PromotionMode,
) -> PromotionStep {
    let PromotionMode::Standard = mode;
    match record {
        RecordState::Absent => PromotionStep::CreateIssue,
        RecordState::Unreadable => {
            PromotionStep::Stop(NotPromoted::CreateOutcomeUnknown)
        }
        RecordState::Present(record) => match &record.stage {
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
        },
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
/// should do.
#[must_use]
pub fn intended_baseline(
    disposition: &PromotionStage,
    draft_digest: &str,
    promoted_digest: &str,
    read_back_matches_promoted: bool,
) -> Option<IntendedBaseline> {
    let baseline = |remote_hash, local_hash: &str| IntendedBaseline {
        remote_hash,
        local_hash: local_hash.to_owned(),
    };
    match disposition {
        PromotionStage::RemoteRetitled { read_back, .. } => Some(baseline(
            RemoteHash::Known(read_back.clone()),
            promoted_digest,
        )),
        PromotionStage::RemoteKept {
            reason: reason @ RemoteKeptReason::UpdateFailed { .. },
            ..
        } => Some(baseline(
            RemoteHash::Known(reason.read_back().clone()),
            draft_digest,
        )),
        PromotionStage::RemoteKept { reason, .. }
            if read_back_matches_promoted =>
        {
            Some(baseline(
                RemoteHash::Known(reason.read_back().clone()),
                draft_digest,
            ))
        }
        PromotionStage::RemoteKept { .. } => {
            Some(baseline(RemoteHash::Unknown, draft_digest))
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
    use crate::sync::RequestFingerprint;

    fn key() -> ExternalId {
        ExternalId::new("PP-900".to_owned())
    }

    fn read_back(hash: &str) -> ReadBack {
        ReadBack {
            hash: hash.to_owned(),
            updated: tracker::RemoteTimestamp::Reported(format!("at-{hash}")),
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
            baseline: IntendedBaseline {
                remote_hash: RemoteHash::Unknown,
                local_hash: "draft".to_owned(),
            },
            recovery_dir: PathBuf::from("retirement-recovery/x"),
            before: Box::new(kept(RemoteKeptReason::NoHash {
                read_back: read_back("r"),
            })),
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
        let known = |hash: &str, local: &str| {
            Some(IntendedBaseline {
                remote_hash: RemoteHash::Known(read_back(hash)),
                local_hash: local.to_owned(),
            })
        };
        let unknown = Some(IntendedBaseline {
            remote_hash: RemoteHash::Unknown,
            local_hash: "draft".to_owned(),
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

        for matches in [false, true] {
            assert_eq!(
                intended_baseline(&retitled, "draft", "promoted", matches),
                known("after", "promoted"),
                "a retitled remote syncs as synced"
            );
            assert_eq!(
                intended_baseline(&update_failed, "draft", "promoted", matches),
                known("r", "draft"),
                "a failed H1 update is pushed by the next sync"
            );
        }
        for stage in &untouched_but_unmatched {
            assert_eq!(
                intended_baseline(stage, "draft", "promoted", true),
                known("r", "draft"),
                "differing only by the ID, the next sync pushes the key H1"
            );
            assert_eq!(
                intended_baseline(stage, "draft", "promoted", false),
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
                intended_baseline(&stage, "draft", "promoted", true),
                None,
                "{stage:?} has not settled the remote side"
            );
        }
    }
}
