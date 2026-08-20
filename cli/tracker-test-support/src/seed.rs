//! Offline-safe guards for the live corpus-seed step.
//!
//! The seed writes real issues to a live tracker, so two decisions must be
//! provably correct before any network call: the target is a known scratch
//! tenant, never production; and a record already seeded is reused rather than
//! duplicated. Both are pure functions here, unit-tested without credentials,
//! so a refactor cannot silently disable the production-write guard.

use std::collections::BTreeSet;

/// The scratch project keys and team ids the seed is permitted to write to.
///
/// Membership is exact after trimming: a target absent from the list is
/// refused, so a mistyped production key is rejected rather than accepted —
/// supplying an identifier is not the same as proving it is not production.
pub struct ScratchAllowlist {
    permitted: BTreeSet<String>,
}

impl ScratchAllowlist {
    /// Parse a comma- or whitespace-separated list of scratch identifiers.
    /// Empty entries are dropped and each is trimmed.
    #[must_use]
    pub fn from_list(raw: &str) -> Self {
        let permitted = raw
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_owned)
            .collect();
        Self { permitted }
    }

    /// Whether the allowlist names no tenant — in which case it permits
    /// nothing, the fail-safe default.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.permitted.is_empty()
    }

    /// Whether `target` is a permitted scratch tenant. Exact match after
    /// trimming; any target absent from the list is refused.
    #[must_use]
    pub fn permits(&self, target: &str) -> bool {
        self.permitted.contains(target.trim())
    }
}

/// Why the seed refused to write.
#[derive(Debug, PartialEq, Eq)]
pub enum SeedRefusal {
    /// The target is absent from the scratch allowlist. Carries the target so
    /// the diagnostic names what was rejected.
    NotAllowlisted(String),
}

/// Refuse unless `target` is a known scratch tenant. Called before any create,
/// so a non-scratch target never reaches the network.
///
/// # Errors
///
/// [`SeedRefusal::NotAllowlisted`] when `target` is absent from `allowlist`.
pub fn guard_target(
    allowlist: &ScratchAllowlist,
    target: &str,
) -> Result<(), SeedRefusal> {
    if allowlist.permits(target) {
        Ok(())
    } else {
        Err(SeedRefusal::NotAllowlisted(target.trim().to_owned()))
    }
}

/// A stable, greppable marker identifying the corpus record an issue was seeded
/// from. Embedded in the seeded issue's body so a re-run finds the existing
/// issue by string match and reuses it.
#[must_use]
pub fn seed_marker(record_id: &str) -> String {
    format!("accelerator-seed:{}", record_id.trim())
}

/// Whether a record still needs seeding, given the markers already present on
/// the scratch tenant. A record whose marker is present is reused, so a
/// repeated run does not accumulate duplicates.
#[must_use]
pub fn needs_seeding(
    record_id: &str,
    existing_markers: &BTreeSet<String>,
) -> bool {
    !existing_markers.contains(&seed_marker(record_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scratch_key_is_accepted() {
        let allowlist = ScratchAllowlist::from_list("SCRATCH, TEST");
        assert!(allowlist.permits("SCRATCH"));
        assert_eq!(guard_target(&allowlist, "SCRATCH"), Ok(()));
    }

    #[test]
    fn a_non_scratch_key_is_rejected() {
        let allowlist = ScratchAllowlist::from_list("SCRATCH, TEST");
        assert!(!allowlist.permits("PROD"));
        assert_eq!(
            guard_target(&allowlist, "PROD"),
            Err(SeedRefusal::NotAllowlisted("PROD".to_owned())),
        );
    }

    #[test]
    fn a_similar_but_mistyped_key_is_rejected() {
        let allowlist = ScratchAllowlist::from_list("SCRATCH");
        assert!(!allowlist.permits("SCRATCHH"));
        assert!(!allowlist.permits("SCRATC"));
        assert!(!allowlist.permits("scratch"));
    }

    #[test]
    fn an_empty_allowlist_permits_nothing() {
        let allowlist = ScratchAllowlist::from_list("");
        assert!(allowlist.is_empty());
        assert!(!allowlist.permits("SCRATCH"));
        assert_eq!(
            guard_target(&allowlist, "SCRATCH"),
            Err(SeedRefusal::NotAllowlisted("SCRATCH".to_owned())),
        );
    }

    #[test]
    fn a_whitespace_variant_matches_after_trim() {
        let allowlist = ScratchAllowlist::from_list("SCRATCH");
        assert!(allowlist.permits("  SCRATCH  "));
    }

    #[test]
    fn from_list_parses_comma_and_whitespace_separators() {
        let allowlist = ScratchAllowlist::from_list("A, B\tC  D,,E ,");
        for key in ["A", "B", "C", "D", "E"] {
            assert!(allowlist.permits(key), "{key} should be permitted");
        }
        assert!(!allowlist.permits("F"));
    }

    #[test]
    fn the_marker_is_stable_and_trims_the_id() {
        assert_eq!(seed_marker("0195"), "accelerator-seed:0195");
        assert_eq!(seed_marker("  0195  "), seed_marker("0195"));
    }

    #[test]
    fn a_second_seed_reuses_rather_than_duplicates() {
        let mut existing = BTreeSet::new();
        assert!(
            needs_seeding("0195", &existing),
            "an unseeded record needs seeding"
        );

        existing.insert(seed_marker("0195"));
        assert!(
            !needs_seeding("0195", &existing),
            "a record already carrying its marker is reused, not re-created"
        );
        assert!(
            needs_seeding("0196", &existing),
            "a different record is still seeded"
        );
    }
}
