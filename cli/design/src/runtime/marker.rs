//! The sticky-failure marker policy: whether a recorded failure suppresses a
//! fresh materialisation attempt this invocation.
//!
//! A crawl makes 100–200 executor invocations. Without negative caching a
//! persistent failure — a full disk, an unwritable cache root, a host too old
//! for the runtime — drives a fresh full-size fetch on every one, and could
//! attempt tens of gigabytes over a single crawl. A marker records the failure
//! so the remaining invocations take the code-only path at once.
//!
//! Two keyings, because the two failure classes clear differently. A fetch or
//! environment failure is keyed to the session and a TTL sized for the crawl
//! bound, so freeing disk or reconnecting is not stranded past the next crawl.
//! A host-condition failure — the runtime materialised but the host cannot run
//! it — is keyed to the resolved tree's digest instead, so a *successful*
//! materialisation does not clear it; only `cache repair` or a digest change
//! does. Every marker is session-scoped for suppression, so a marker an
//! untrusted repository committed cannot suppress this session's findings.

use crate::runtime::downgrade::DowngradeReason;

/// A recorded materialisation or host-condition failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub reason: DowngradeReason,
    /// The session that recorded it. Only the recording session's own markers
    /// suppress, so a marker committed by an untrusted repository cannot.
    pub session: String,
    /// Clock seconds at which it was recorded, for the TTL of a fetch or
    /// environment marker.
    pub recorded_at: u64,
    /// The resolved tree digest for a host-condition marker; `None` for a fetch
    /// or environment marker, which is TTL-bound instead.
    pub digest: Option<String>,
}

/// Whether a fresh attempt is suppressed, and to what reason, given the current
/// session, clock and resolved digest.
#[must_use]
pub fn suppresses(
    marker: &Marker,
    session: &str,
    now: u64,
    ttl_seconds: u64,
    current_digest: Option<&str>,
) -> Option<DowngradeReason> {
    if marker.session != session {
        return None;
    }
    marker.digest.as_deref().map_or_else(
        || {
            (now.saturating_sub(marker.recorded_at) < ttl_seconds)
                .then_some(marker.reason)
        },
        |digest| (current_digest == Some(digest)).then_some(marker.reason),
    )
}

/// Whether a successful materialisation clears this marker.
///
/// A fetch or environment marker is cleared — the condition it recorded has
/// lifted — but a host-condition marker is not, since materialisation
/// succeeding says nothing about whether the host can run what it materialised.
#[must_use]
pub const fn cleared_by_successful_ensure(marker: &Marker) -> bool {
    marker.digest.is_none()
}

/// Whether a downgrade reason is a host condition keyed to the tree digest.
///
/// These arise only *after* a fetch — the bundled runtime materialised but the
/// host cannot run it — and are expensive to re-attempt (a full spawn plus the
/// readiness timeout), so a marker for one outlives a successful materialisation.
#[must_use]
pub const fn is_host_condition(reason: DowngradeReason) -> bool {
    matches!(
        reason,
        DowngradeReason::GlibcTooOld
            | DowngradeReason::RuntimeLibrariesMissing
            | DowngradeReason::LoaderUnresolvable
    )
}

#[cfg(test)]
mod tests {
    use super::cleared_by_successful_ensure;
    use super::is_host_condition;
    use super::suppresses;
    use super::Marker;
    use crate::runtime::downgrade::DowngradeReason;

    const TTL: u64 = 300;

    fn fetch_marker() -> Marker {
        Marker {
            reason: DowngradeReason::DiskFloorNotMet,
            session: "S".to_owned(),
            recorded_at: 1000,
            digest: None,
        }
    }

    fn host_marker() -> Marker {
        Marker {
            reason: DowngradeReason::GlibcTooOld,
            session: "S".to_owned(),
            recorded_at: 1000,
            digest: Some("deadbeef".to_owned()),
        }
    }

    #[test]
    fn a_fetch_marker_suppresses_within_the_ttl_for_its_own_session() {
        assert_eq!(
            suppresses(&fetch_marker(), "S", 1200, TTL, None),
            Some(DowngradeReason::DiskFloorNotMet)
        );
    }

    #[test]
    fn a_fetch_marker_past_its_ttl_does_not_suppress() {
        assert_eq!(
            suppresses(&fetch_marker(), "S", 1000 + TTL, TTL, None),
            None
        );
    }

    #[test]
    fn a_marker_from_another_session_never_suppresses() {
        // The pre-planted / committed marker defence.
        assert_eq!(suppresses(&fetch_marker(), "OTHER", 1000, TTL, None), None);
        let host = Marker {
            session: "OTHER".to_owned(),
            ..host_marker()
        };
        assert_eq!(suppresses(&host, "S", 1000, TTL, Some("deadbeef")), None);
    }

    #[test]
    fn a_host_marker_suppresses_while_the_digest_is_unchanged() {
        assert_eq!(
            suppresses(&host_marker(), "S", 9_999_999, TTL, Some("deadbeef")),
            Some(DowngradeReason::GlibcTooOld),
            "a host marker ignores the TTL and holds while the digest holds"
        );
    }

    #[test]
    fn a_host_marker_stops_suppressing_when_the_digest_changes() {
        assert_eq!(
            suppresses(&host_marker(), "S", 1000, TTL, Some("cafef00d")),
            None
        );
    }

    #[test]
    fn a_successful_ensure_clears_a_fetch_marker_but_not_a_host_marker() {
        assert!(cleared_by_successful_ensure(&fetch_marker()));
        assert!(!cleared_by_successful_ensure(&host_marker()));
    }

    #[test]
    fn host_conditions_are_the_post_fetch_reasons() {
        for reason in [
            DowngradeReason::GlibcTooOld,
            DowngradeReason::RuntimeLibrariesMissing,
            DowngradeReason::LoaderUnresolvable,
        ] {
            assert!(is_host_condition(reason), "{reason}");
        }
        for reason in [
            DowngradeReason::DiskFloorNotMet,
            DowngradeReason::CacheUnwritable,
            DowngradeReason::ArtifactUnavailable,
            DowngradeReason::MaterialisationInProgress,
        ] {
            assert!(!is_host_condition(reason), "{reason}");
        }
    }
}
