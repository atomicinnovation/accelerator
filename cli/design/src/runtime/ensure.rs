//! Mapping the launcher's `cache ensure` failure cause onto a downgrade reason.
//!
//! `ensure` is a separately-built executable; its failure envelope carries an
//! enumerated `cause` token. The executor maps the token here — never the
//! human-readable message — so a taxonomy change is a compile-checked table
//! rather than a substring match, and an untrusted stream cannot force a
//! downgrade by embedding a marker. An unrecognised cause, or an ensure that
//! produced no parseable envelope, maps to `artifact-unavailable`: a
//! diagnosable default, and sticky like the persistent causes it stands in for.

use crate::runtime::downgrade::DowngradeReason;

/// A downgrade reason and whether it is a persistent host or environment
/// condition that must suppress re-attempts for the rest of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnsureVerdict {
    pub reason: DowngradeReason,
    pub sticky: bool,
}

/// Map a launcher ensure-cause token onto a downgrade reason.
///
/// `materialisation-in-progress` is the one cause that is **not** sticky:
/// another process is actively fetching and will succeed shortly, so the next
/// invocation must retry rather than degrade the rest of the crawl.
#[must_use]
pub fn classify_cause(cause: &str) -> EnsureVerdict {
    match cause {
        "disk-shortfall" => EnsureVerdict {
            reason: DowngradeReason::DiskFloorNotMet,
            sticky: true,
        },
        "cache-unwritable" => EnsureVerdict {
            reason: DowngradeReason::CacheUnwritable,
            sticky: true,
        },
        "materialisation-in-progress" => EnsureVerdict {
            reason: DowngradeReason::MaterialisationInProgress,
            sticky: false,
        },
        // Unreachable host, a signature or digest mismatch, an absent artifact,
        // or an unrecognised cause: the artifacts could not be materialised, and
        // a re-attempt within the crawl would repeat the same full-size failure.
        _ => EnsureVerdict {
            reason: DowngradeReason::ArtifactUnavailable,
            sticky: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::classify_cause;
    use crate::runtime::downgrade::DowngradeReason;

    #[test]
    fn a_disk_shortfall_maps_to_disk_floor_not_met_and_is_sticky() {
        let verdict = classify_cause("disk-shortfall");
        assert_eq!(verdict.reason, DowngradeReason::DiskFloorNotMet);
        assert!(verdict.sticky);
    }

    #[test]
    fn an_unwritable_cache_maps_to_cache_unwritable_and_is_sticky() {
        let verdict = classify_cause("cache-unwritable");
        assert_eq!(verdict.reason, DowngradeReason::CacheUnwritable);
        assert!(verdict.sticky);
    }

    #[test]
    fn materialisation_in_progress_is_the_one_non_sticky_cause() {
        let verdict = classify_cause("materialisation-in-progress");
        assert_eq!(verdict.reason, DowngradeReason::MaterialisationInProgress);
        assert!(
            !verdict.sticky,
            "another process is fetching; retry next time"
        );
    }

    #[test]
    fn transport_and_integrity_failures_map_to_artifact_unavailable_sticky() {
        for cause in [
            "unreachable",
            "signature-mismatch",
            "digest-mismatch",
            "artifact-unavailable",
        ] {
            let verdict = classify_cause(cause);
            assert_eq!(
                verdict.reason,
                DowngradeReason::ArtifactUnavailable,
                "{cause}"
            );
            assert!(verdict.sticky, "{cause}");
        }
    }

    #[test]
    fn an_unknown_cause_falls_back_to_artifact_unavailable_sticky() {
        let verdict = classify_cause("something-new-the-launcher-added");
        assert_eq!(verdict.reason, DowngradeReason::ArtifactUnavailable);
        assert!(verdict.sticky);
    }
}
