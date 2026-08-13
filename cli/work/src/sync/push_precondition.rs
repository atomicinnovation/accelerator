//! The pending-push marker's decision table: whether `create --push` may
//! proceed, reuse a previous run's id, or must refuse.
//!
//! A wrong branch here binds two local work items to one remote issue, a
//! failure neither VCS revert nor a re-run recovers from.

use tracker::ExternalId;

/// A create attempt's fingerprint: proves two requests are the same before
/// a marker's id is adopted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestFingerprint {
    pub title: String,
    pub digest: String,
    pub attempted_at: u64,
    pub failure: Option<String>,
}

/// A marker's persisted state: a create was attempted (outcome unknown), or
/// it succeeded and the local write had not happened yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingPush {
    Attempted {
        request: RequestFingerprint,
    },
    Created {
        request: RequestFingerprint,
        external_id: ExternalId,
    },
}

/// The marker as read from disk.
///
/// `Absent` and `Unreadable` stay distinct: conflating them lets a crash
/// mid-write — exactly what the marker exists to survive — read as "no
/// previous attempt" and re-issue a non-idempotent `create`.
pub enum MarkerState<'a> {
    Absent,
    Unreadable,
    Present(&'a PendingPush),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    MarkerUnreadable,
    PriorAttemptUnknownOutcome,
    FingerprintMismatch,
    AlreadyWritten,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushPrecondition {
    Proceed,
    ReuseId(ExternalId),
    Refuse(RefusalReason),
}

/// Decides whether a `create --push` attempt may proceed.
///
/// `corpus_carries` reports whether a work item on disk already carries a
/// given `external_id`, closing the hazard the fingerprint alone cannot: a
/// crash between the local write and the marker delete leaves a `Created`
/// marker whose fingerprint still matches, and a re-run would then write a
/// second file carrying the same `external_id`.
#[must_use]
pub fn push_precondition(
    marker: &MarkerState<'_>,
    request_digest: &str,
    corpus_carries: &dyn Fn(&ExternalId) -> bool,
) -> PushPrecondition {
    match marker {
        MarkerState::Absent => PushPrecondition::Proceed,
        MarkerState::Unreadable => {
            PushPrecondition::Refuse(RefusalReason::MarkerUnreadable)
        }
        MarkerState::Present(PendingPush::Attempted { .. }) => {
            PushPrecondition::Refuse(RefusalReason::PriorAttemptUnknownOutcome)
        }
        MarkerState::Present(PendingPush::Created {
            request,
            external_id,
        }) => {
            if request.digest != request_digest {
                return PushPrecondition::Refuse(
                    RefusalReason::FingerprintMismatch,
                );
            }
            if corpus_carries(external_id) {
                PushPrecondition::Refuse(RefusalReason::AlreadyWritten)
            } else {
                PushPrecondition::ReuseId(external_id.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::push_precondition;
    use super::MarkerState;
    use super::PendingPush;
    use super::PushPrecondition;
    use super::RefusalReason;
    use super::RequestFingerprint;
    use tracker::ExternalId;

    fn fingerprint(digest: &str) -> RequestFingerprint {
        RequestFingerprint {
            title: "Title".to_owned(),
            digest: digest.to_owned(),
            attempted_at: 0,
            failure: None,
        }
    }

    #[test]
    fn absent_proceeds() {
        assert_eq!(
            push_precondition(&MarkerState::Absent, "d", &|_| false),
            PushPrecondition::Proceed
        );
    }

    #[test]
    fn unreadable_refuses() {
        assert_eq!(
            push_precondition(&MarkerState::Unreadable, "d", &|_| false),
            PushPrecondition::Refuse(RefusalReason::MarkerUnreadable)
        );
    }

    #[test]
    fn attempted_refuses_as_unknown_outcome() {
        let marker = PendingPush::Attempted {
            request: fingerprint("d"),
        };
        assert_eq!(
            push_precondition(&MarkerState::Present(&marker), "d", &|_| false),
            PushPrecondition::Refuse(RefusalReason::PriorAttemptUnknownOutcome)
        );
    }

    #[test]
    fn matching_created_with_the_id_absent_from_the_corpus_reuses_it() {
        let id = ExternalId::new("ENG-1".to_owned());
        let marker = PendingPush::Created {
            request: fingerprint("d"),
            external_id: id.clone(),
        };
        assert_eq!(
            push_precondition(&MarkerState::Present(&marker), "d", &|_| false),
            PushPrecondition::ReuseId(id)
        );
    }

    #[test]
    fn matching_created_with_the_id_present_refuses_as_already_written() {
        let id = ExternalId::new("ENG-1".to_owned());
        let marker = PendingPush::Created {
            request: fingerprint("d"),
            external_id: id,
        };
        assert_eq!(
            push_precondition(&MarkerState::Present(&marker), "d", &|_| true),
            PushPrecondition::Refuse(RefusalReason::AlreadyWritten)
        );
    }

    #[test]
    fn a_fingerprint_mismatch_refuses_without_adopting_the_id() {
        let id = ExternalId::new("ENG-1".to_owned());
        let marker = PendingPush::Created {
            request: fingerprint("old-digest"),
            external_id: id,
        };
        assert_eq!(
            push_precondition(
                &MarkerState::Present(&marker),
                "new-digest",
                &|_| false
            ),
            PushPrecondition::Refuse(RefusalReason::FingerprintMismatch)
        );
    }
}
