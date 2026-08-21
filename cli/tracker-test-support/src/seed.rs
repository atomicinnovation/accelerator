//! Offline-safe guards for the live corpus-seed step.
//!
//! The seed writes real issues to a live tracker, so two decisions must be
//! provably correct before any network call: the target is a known scratch
//! tenant, never production; and a record already seeded is reused rather than
//! duplicated. Both are pure functions here, unit-tested without credentials,
//! so a refactor cannot silently disable the production-write guard.

use std::collections::BTreeSet;

use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker::TrackerError;

/// The prefix every seed marker carries, so a marker is recognisable in an
/// issue body a `show` returns.
const MARKER_PREFIX: &str = "accelerator-seed:";

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
    format!("{MARKER_PREFIX}{}", record_id.trim())
}

/// The seed markers present in an issue body — the whitespace-delimited tokens
/// carrying the marker prefix. A seeded body carries its marker on its own
/// line, so a token match recovers it.
pub fn markers_in(body: &str) -> impl Iterator<Item = &str> {
    body.split_whitespace()
        .filter(|token| token.starts_with(MARKER_PREFIX))
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

/// One record to seed onto the scratch tenant. `body` is the content before the
/// marker; `run_seed` appends the marker so a re-run can recognise the issue.
pub struct SeedRecord {
    pub id: String,
    pub title: String,
    pub body: String,
    pub kind: String,
}

/// What a seed run did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SeedSummary {
    /// Record ids for which a new remote issue was created.
    pub created: Vec<String>,
    /// Record ids already present on the tenant, left untouched.
    pub reused: Vec<String>,
}

/// A small representative corpus for the live seed. Includes one record with
/// an empty description so the classification exercise covers the
/// absent-description projection path per provider.
#[must_use]
pub fn representative_records() -> Vec<SeedRecord> {
    vec![
        SeedRecord {
            id: "seed-a".to_owned(),
            title: "Seed A: a described task".to_owned(),
            body: "A seeded item carrying a description.".to_owned(),
            kind: "task".to_owned(),
        },
        SeedRecord {
            id: "seed-b".to_owned(),
            title: "Seed B: another described task".to_owned(),
            body: "A second seeded item with a description.".to_owned(),
            kind: "task".to_owned(),
        },
        SeedRecord {
            id: "seed-c".to_owned(),
            title: "Seed C: no description".to_owned(),
            body: String::new(),
            kind: "task".to_owned(),
        },
    ]
}

/// Why a seed run stopped.
#[derive(Debug)]
pub enum SeedError {
    /// The discovery `search` failed.
    Discovery(TrackerError),
    /// The discovery was truncated. Seeding on a lower-bound view could
    /// re-create an already-seeded issue the query did not see, so the run
    /// refuses rather than risk a duplicate — narrow the scope and retry.
    Incomplete,
    /// A `show` of a discovered issue failed.
    Show(TrackerError),
    /// A `create` of a missing record failed.
    Create(TrackerError),
}

/// Seed the scratch tenant with one issue per record, idempotently.
///
/// Discovers the markers already present (one `search`, then a `show` per
/// discovered issue), then `create`s only the records whose marker is absent.
/// A repeated run reuses rather than duplicates.
///
/// # Errors
///
/// [`SeedError::Incomplete`] when the discovery is truncated (refusing to risk
/// a duplicate); [`SeedError::Discovery`]/[`SeedError::Show`]/
/// [`SeedError::Create`] when the corresponding port call fails.
pub fn run_seed(
    tracker: &dyn RemoteTracker,
    scope: &SearchScope,
    records: &[SeedRecord],
) -> Result<SeedSummary, SeedError> {
    let discovery = tracker.search(scope).map_err(SeedError::Discovery)?;
    if !discovery.complete {
        return Err(SeedError::Incomplete);
    }

    let mut existing: BTreeSet<String> = BTreeSet::new();
    for (id, _stamp) in &discovery.found {
        let issue = tracker.show(id).map_err(SeedError::Show)?;
        for marker in markers_in(&issue.body) {
            existing.insert(marker.to_owned());
        }
    }

    let mut summary = SeedSummary::default();
    for record in records {
        if needs_seeding(&record.id, &existing) {
            let body =
                format!("{}\n\n{}", record.body, seed_marker(&record.id));
            tracker
                .create(&record.title, &body, &record.kind)
                .map_err(SeedError::Create)?;
            summary.created.push(record.id.clone());
        } else {
            summary.reused.push(record.id.clone());
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use tracker::ExternalId;
    use tracker::RemoteIssue;
    use tracker::RemoteTimestamp;

    use super::*;
    use crate::Call;
    use crate::RecordingTracker;

    fn scope() -> SearchScope {
        SearchScope {
            project: Some("SCR".to_owned()),
            all_projects: false,
            filters: Vec::new(),
        }
    }

    fn record(id: &str, kind: &str) -> SeedRecord {
        SeedRecord {
            id: id.to_owned(),
            title: format!("issue {id}"),
            body: format!("body {id}"),
            kind: kind.to_owned(),
        }
    }

    fn create_count(tracker: &RecordingTracker) -> usize {
        tracker
            .calls()
            .into_iter()
            .filter(|call| matches!(call, Call::Create { .. }))
            .count()
    }

    #[test]
    fn markers_in_extracts_seed_tokens() {
        let body = "Title\nsome prose\n\naccelerator-seed:0195\n";
        let found: Vec<_> = markers_in(body).collect();
        assert_eq!(found, vec!["accelerator-seed:0195"]);
    }

    #[test]
    fn seeds_every_record_when_none_exist() {
        let tracker =
            RecordingTracker::holding(Vec::new()).discovering(Vec::new(), true);
        let records = [record("0195", "task"), record("0196", "bug")];

        let summary =
            run_seed(&tracker, &scope(), &records).expect("seed runs");

        assert_eq!(summary.created, vec!["0195", "0196"]);
        assert!(summary.reused.is_empty());
        assert_eq!(create_count(&tracker), 2);
        for call in tracker.calls() {
            if let Call::Create { body, .. } = call {
                assert!(
                    markers_in(&body).next().is_some(),
                    "a created body must carry its marker so a re-run reuses it"
                );
            }
        }
    }

    #[test]
    fn reuses_records_already_seeded() {
        let seeded = ExternalId::new("SCR-1".to_owned());
        let issue = RemoteIssue {
            updated: RemoteTimestamp::NotReported,
            body: format!("issue 0195\nbody\n\n{}", seed_marker("0195")),
        };
        let tracker = RecordingTracker::holding(vec![(seeded.clone(), issue)])
            .discovering(vec![(seeded, RemoteTimestamp::NotReported)], true);
        let records = [record("0195", "task"), record("0196", "bug")];

        let summary =
            run_seed(&tracker, &scope(), &records).expect("seed runs");

        assert_eq!(summary.reused, vec!["0195"]);
        assert_eq!(summary.created, vec!["0196"]);
        assert_eq!(create_count(&tracker), 1);
    }

    #[test]
    fn a_truncated_discovery_refuses_to_avoid_duplicates() {
        let tracker = RecordingTracker::holding(Vec::new())
            .discovering(Vec::new(), false);
        let records = [record("0195", "task")];

        let result = run_seed(&tracker, &scope(), &records);

        assert!(matches!(result, Err(SeedError::Incomplete)));
        assert_eq!(create_count(&tracker), 0);
    }

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
