//! The pending-push marker's decision table: whether `create --push` may
//! proceed, reuse a previous run's id, or must refuse.
//!
//! A wrong branch here binds two local work items to one remote issue, a
//! failure neither VCS revert nor a re-run recovers from.

use tracker::ExternalId;

use crate::tracker_key::TrackerKey;

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

impl RefusalReason {
    /// The single spelling of this reason wherever a refusal is reported.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::MarkerUnreadable => "marker-unreadable",
            Self::PriorAttemptUnknownOutcome => "prior-attempt-unknown",
            Self::FingerprintMismatch => "fingerprint-mismatch",
            Self::AlreadyWritten => "already-written",
        }
    }
}

/// How a person settles a refused pending push, having searched the
/// tracker for the item's issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingPushOrder {
    /// The item's issue exists under this key.
    Adopt(TrackerKey),
    /// No issue exists for the item, so it is created anew.
    Create,
}

impl std::fmt::Display for PendingPushOrder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Adopt(key) => write!(formatter, "adopt:{key}"),
            Self::Create => formatter.write_str("create"),
        }
    }
}

/// An `adopt:` order whose key is not shaped like a tracker key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotATrackerKey(pub String);

/// The pending-push order a `--resolve` token names, folded as conflict
/// orders are; `None` when the token names no pending-push order.
#[must_use]
pub fn resolve_pending_push_token(
    raw: &str,
) -> Option<Result<PendingPushOrder, NotATrackerKey>> {
    let trimmed = raw.trim_matches(|c: char| c.is_ascii_whitespace());
    if trimmed.eq_ignore_ascii_case("create") {
        return Some(Ok(PendingPushOrder::Create));
    }
    let (verb, key) = trimmed.split_once(':')?;
    if !verb.eq_ignore_ascii_case("adopt") {
        return None;
    }
    Some(
        TrackerKey::parse(key)
            .map(PendingPushOrder::Adopt)
            .ok_or_else(|| NotATrackerKey(key.to_owned())),
    )
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
    use super::resolve_pending_push_token;
    use super::MarkerState;
    use super::NotATrackerKey;
    use super::PendingPush;
    use super::PendingPushOrder;
    use super::PushPrecondition;
    use super::RefusalReason;
    use super::RequestFingerprint;
    use tracker::ExternalId;

    use crate::tracker_key::TrackerKey;

    fn key(raw: &str) -> TrackerKey {
        TrackerKey::parse(raw).unwrap_or_else(|| unreachable!("a tracker key"))
    }

    fn fingerprint(digest: &str) -> RequestFingerprint {
        RequestFingerprint {
            title: "Title".to_owned(),
            digest: digest.to_owned(),
            attempted_at: 0,
            failure: None,
        }
    }

    #[test]
    fn each_refusal_reason_has_one_token() {
        for (reason, token) in [
            (RefusalReason::MarkerUnreadable, "marker-unreadable"),
            (
                RefusalReason::PriorAttemptUnknownOutcome,
                "prior-attempt-unknown",
            ),
            (RefusalReason::FingerprintMismatch, "fingerprint-mismatch"),
            (RefusalReason::AlreadyWritten, "already-written"),
        ] {
            assert_eq!(reason.token(), token);
        }
    }

    #[test]
    fn adopt_names_the_key_it_links() {
        assert_eq!(
            resolve_pending_push_token(" Adopt:PP-12 "),
            Some(Ok(PendingPushOrder::Adopt(key("PP-12"))))
        );
    }

    #[test]
    fn create_is_folded_like_a_conflict_order() {
        assert_eq!(
            resolve_pending_push_token("CREATE"),
            Some(Ok(PendingPushOrder::Create))
        );
    }

    #[test]
    fn an_adopt_whose_key_is_not_a_tracker_key_is_malformed() {
        assert_eq!(
            resolve_pending_push_token("adopt:../x"),
            Some(Err(NotATrackerKey("../x".to_owned())))
        );
    }

    #[test]
    fn a_conflict_order_is_not_a_pending_push_order() {
        assert_eq!(resolve_pending_push_token("remote"), None);
    }

    #[test]
    fn an_order_spells_itself_as_its_token() {
        assert_eq!(
            PendingPushOrder::Adopt(key("PP-12")).to_string(),
            "adopt:PP-12"
        );
        assert_eq!(PendingPushOrder::Create.to_string(), "create");
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
