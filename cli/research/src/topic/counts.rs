//! The counts a set's files give, and whether its manifest and synthesis
//! agree with them.

use crate::topic::layout::entry::is_finding_entry;
use crate::topic::layout::entry::is_report_entry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountedFindings {
    names: Vec<String>,
    round_count: u32,
}

impl CountedFindings {
    /// # Errors
    ///
    /// Returns the first error `round_of` returns, which ends the count.
    pub fn of<E>(
        entries: &[&str],
        mut round_of: impl FnMut(&str) -> Result<Option<u32>, E>,
    ) -> Result<Self, E> {
        let mut names = Vec::new();
        let mut round_count = 0;
        for name in entries.iter().filter(|name| is_finding_entry(name)) {
            if let Some(round) = round_of(name)? {
                round_count = round_count.max(round);
            }
            names.push((*name).to_owned());
        }
        Ok(Self { names, round_count })
    }

    #[must_use]
    pub fn names(&self) -> &[String] {
        &self.names
    }

    #[must_use]
    pub const fn round_count(&self) -> u32 {
        self.round_count
    }

    #[must_use]
    pub fn finding_count(&self) -> u32 {
        count_of(self.names.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountedReports {
    names: Vec<String>,
}

impl CountedReports {
    #[must_use]
    pub fn of(entries: &[&str]) -> Self {
        let names = entries
            .iter()
            .filter(|name| is_report_entry(name))
            .map(|name| (*name).to_owned())
            .collect();
        Self { names }
    }

    #[must_use]
    pub fn names(&self) -> &[String] {
        &self.names
    }

    #[must_use]
    pub fn report_count(&self) -> u32 {
        count_of(self.names.len())
    }
}

fn count_of(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredCount {
    Count(u32),
    Missing,
    Unreadable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCounts {
    pub round_count: StoredCount,
    pub finding_count: StoredCount,
    pub report_count: StoredCount,
    pub rounds_covered: StoredCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestCount {
    RoundCount,
    FindingCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountField {
    Manifest(ManifestCount),
    RoundsCovered,
}

impl CountField {
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Manifest(ManifestCount::RoundCount) => "round_count",
            Self::Manifest(ManifestCount::FindingCount) => "finding_count",
            Self::RoundsCovered => "rounds_covered",
        }
    }

    #[must_use]
    pub const fn document(self) -> &'static str {
        match self {
            Self::Manifest(_) => "manifest.md",
            Self::RoundsCovered => "synthesis.md",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mismatch {
    Missing {
        count: ManifestCount,
    },
    Unreadable {
        field: CountField,
        raw: String,
    },
    Disagrees {
        count: ManifestCount,
        stored: u32,
        derived: u32,
    },
    UncoveredRound {
        derived: u32,
        covered: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Freshness {
    pub mismatches: Vec<Mismatch>,
}

impl Freshness {
    #[must_use]
    pub fn assess(stored: &StoredCounts, findings: &CountedFindings) -> Self {
        let mismatches = [
            compare(
                ManifestCount::RoundCount,
                &stored.round_count,
                findings.round_count(),
            ),
            compare(
                ManifestCount::FindingCount,
                &stored.finding_count,
                findings.finding_count(),
            ),
            uncovered(&stored.rounds_covered, findings.round_count()),
        ]
        .into_iter()
        .flatten()
        .collect();
        Self { mismatches }
    }

    #[must_use]
    pub const fn is_fresh(&self) -> bool {
        self.mismatches.is_empty()
    }
}

fn compare(
    count: ManifestCount,
    stored: &StoredCount,
    derived: u32,
) -> Option<Mismatch> {
    match stored {
        StoredCount::Count(value) if *value == derived => None,
        StoredCount::Count(value) => Some(Mismatch::Disagrees {
            count,
            stored: *value,
            derived,
        }),
        StoredCount::Missing => Some(Mismatch::Missing { count }),
        StoredCount::Unreadable(raw) => Some(Mismatch::Unreadable {
            field: CountField::Manifest(count),
            raw: raw.clone(),
        }),
    }
}

fn uncovered(rounds_covered: &StoredCount, derived: u32) -> Option<Mismatch> {
    let covered = match rounds_covered {
        StoredCount::Count(covered) => *covered,
        StoredCount::Missing => 0,
        StoredCount::Unreadable(raw) => {
            return Some(Mismatch::Unreadable {
                field: CountField::RoundsCovered,
                raw: raw.clone(),
            });
        }
    };
    (derived > covered).then_some(Mismatch::UncoveredRound { derived, covered })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountsReading {
    pub stored: StoredCounts,
    pub findings: CountedFindings,
    pub reports: CountedReports,
}

impl CountsReading {
    #[must_use]
    pub fn freshness(&self) -> Freshness {
        Freshness::assess(&self.stored, &self.findings)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::convert::Infallible;

    use super::CountField;
    use super::CountedFindings;
    use super::CountedReports;
    use super::Freshness;
    use super::ManifestCount;
    use super::Mismatch;
    use super::StoredCount;
    use super::StoredCounts;

    fn findings(rounds: &[(&str, Option<u32>)]) -> CountedFindings {
        let names: Vec<&str> = rounds.iter().map(|(name, _)| *name).collect();
        let round_of = |name: &str| -> Result<Option<u32>, Infallible> {
            Ok(rounds
                .iter()
                .find(|(entry, _)| *entry == name)
                .and_then(|(_, round)| *round))
        };
        match CountedFindings::of(&names, round_of) {
            Ok(counted) => counted,
            Err(never) => match never {},
        }
    }

    fn two_rounds_of_two_findings() -> CountedFindings {
        findings(&[("01-a-web.md", Some(1)), ("02-b-web.md", Some(2))])
    }

    fn stored(
        round_count: StoredCount,
        finding_count: StoredCount,
        rounds_covered: StoredCount,
    ) -> StoredCounts {
        StoredCounts {
            round_count,
            finding_count,
            report_count: StoredCount::Missing,
            rounds_covered,
        }
    }

    fn agreeing() -> StoredCounts {
        stored(
            StoredCount::Count(2),
            StoredCount::Count(2),
            StoredCount::Count(2),
        )
    }

    #[test]
    fn the_count_rule_counts_visible_numbered_markdown_findings() {
        let counted = findings(&[
            ("01-a-web.md", Some(1)),
            ("02-b-web.md", Some(2)),
            (".03-c-web.md.invalid", Some(3)),
            ("01-a-web.levels", Some(4)),
            ("notes.md", Some(5)),
        ]);

        assert_eq!(counted.names(), ["01-a-web.md", "02-b-web.md"]);
        assert_eq!(counted.round_count(), 2);
        assert_eq!(counted.finding_count(), 2);
    }

    #[test]
    fn only_a_counted_finding_s_round_is_read() {
        let asked = RefCell::new(Vec::new());
        let entries = [
            "01-a-web.md",
            ".03-c-web.md.invalid",
            "01-a-web.levels",
            "notes.md",
            "02-b-web.md",
        ];

        let counted = CountedFindings::of(&entries, |name| {
            asked.borrow_mut().push(name.to_owned());
            Ok::<_, Infallible>(Some(1))
        });

        assert!(counted.is_ok());
        assert_eq!(asked.into_inner(), ["01-a-web.md", "02-b-web.md"]);
    }

    #[test]
    fn a_markdown_extension_counts_in_any_case() {
        let counted = findings(&[("07-a-web.MD", Some(1))]);

        assert_eq!(counted.finding_count(), 1);
    }

    #[test]
    fn a_finding_without_a_round_counts_but_reaches_no_round() {
        let counted =
            findings(&[("01-a-web.md", Some(1)), ("02-b-web.md", None)]);

        assert_eq!(counted.finding_count(), 2);
        assert_eq!(counted.round_count(), 1);
    }

    #[test]
    fn a_failed_round_read_propagates() {
        let counted =
            CountedFindings::of(&["01-a-web.md"], |_| Err("unreadable"));

        assert_eq!(counted, Err("unreadable"));
    }

    #[test]
    fn a_set_without_findings_reaches_round_zero() {
        let counted = findings(&[]);

        assert_eq!(counted.round_count(), 0);
        assert_eq!(counted.finding_count(), 0);
    }

    #[test]
    fn the_counted_reports_are_the_visible_markdown_reports() {
        let counted =
            CountedReports::of(&["a.md", "b-2.md", ".c.md", "notes.txt"]);

        assert_eq!(counted.names(), ["a.md", "b-2.md"]);
        assert_eq!(counted.report_count(), 2);
        assert_eq!(CountedReports::of(&[]).report_count(), 0);
    }

    #[test]
    fn a_set_whose_counts_agree_is_fresh() {
        let freshness =
            Freshness::assess(&agreeing(), &two_rounds_of_two_findings());

        assert!(freshness.is_fresh(), "{freshness:?}");
    }

    #[test]
    fn each_disagreement_is_its_own_mismatch() {
        let stale = stored(
            StoredCount::Count(3),
            StoredCount::Count(9),
            StoredCount::Count(1),
        );

        let freshness =
            Freshness::assess(&stale, &two_rounds_of_two_findings());

        assert_eq!(
            freshness.mismatches,
            [
                Mismatch::Disagrees {
                    count: ManifestCount::RoundCount,
                    stored: 3,
                    derived: 2,
                },
                Mismatch::Disagrees {
                    count: ManifestCount::FindingCount,
                    stored: 9,
                    derived: 2,
                },
                Mismatch::UncoveredRound {
                    derived: 2,
                    covered: 1,
                },
            ]
        );
    }

    #[test]
    fn an_unreadable_stored_count_is_a_mismatch() {
        let stale = stored(
            StoredCount::Unreadable("2".to_owned()),
            StoredCount::Missing,
            StoredCount::Count(2),
        );

        let freshness =
            Freshness::assess(&stale, &two_rounds_of_two_findings());

        assert_eq!(
            freshness.mismatches,
            [
                Mismatch::Unreadable {
                    field: CountField::Manifest(ManifestCount::RoundCount),
                    raw: "2".to_owned(),
                },
                Mismatch::Missing {
                    count: ManifestCount::FindingCount,
                },
            ]
        );
    }

    #[test]
    fn an_unreadable_rounds_covered_is_a_mismatch() {
        let stale = stored(
            StoredCount::Count(2),
            StoredCount::Count(2),
            StoredCount::Unreadable("two".to_owned()),
        );

        let freshness =
            Freshness::assess(&stale, &two_rounds_of_two_findings());

        assert_eq!(
            freshness.mismatches,
            [Mismatch::Unreadable {
                field: CountField::RoundsCovered,
                raw: "two".to_owned(),
            }]
        );
    }

    #[test]
    fn a_missing_rounds_covered_covers_round_zero() {
        let unsynthesised = stored(
            StoredCount::Count(2),
            StoredCount::Count(2),
            StoredCount::Missing,
        );

        let freshness =
            Freshness::assess(&unsynthesised, &two_rounds_of_two_findings());
        let empty = Freshness::assess(
            &stored(
                StoredCount::Count(0),
                StoredCount::Count(0),
                StoredCount::Missing,
            ),
            &findings(&[]),
        );

        assert_eq!(
            freshness.mismatches,
            [Mismatch::UncoveredRound {
                derived: 2,
                covered: 0,
            }]
        );
        assert!(empty.is_fresh(), "{empty:?}");
    }

    #[test]
    fn a_synthesis_covering_the_highest_round_is_fresh() {
        let ahead = stored(
            StoredCount::Count(2),
            StoredCount::Count(2),
            StoredCount::Count(3),
        );

        let equal =
            Freshness::assess(&agreeing(), &two_rounds_of_two_findings());
        let beyond = Freshness::assess(&ahead, &two_rounds_of_two_findings());

        assert!(equal.is_fresh(), "{equal:?}");
        assert!(beyond.is_fresh(), "{beyond:?}");
    }
}
