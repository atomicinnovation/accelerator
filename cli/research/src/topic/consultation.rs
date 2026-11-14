//! Whether `ask` and `report` may consult a set: three gates, applied in
//! order.

use crate::topic::counts::CountsReading;
use crate::topic::counts::Mismatch;
use crate::topic::outline::Pair;
use crate::topic::plan::RoundPlan;
use crate::topic::plan::Skip;

const CONSULTABLE_STATUSES: [&str; 2] = ["synthesised", "complete"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Consultation {
    Open(Documents),
    Refused(Refusal),
}

/// The set-relative paths a consultation reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Documents {
    pub findings: Vec<String>,
    pub reports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    Status {
        status: String,
    },
    Freshness {
        mismatches: Vec<Mismatch>,
    },
    Conducted {
        unanswered: Vec<Pair>,
        held_back: Vec<HeldBackItem>,
    },
}

/// An outline item that is not complete although no pair of it is left to
/// research, with the pairs of it `conduct` skips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldBackItem {
    pub question: String,
    pub skips: Vec<Skip>,
}

impl Consultation {
    /// # Errors
    ///
    /// Returns the error of the first reader that fails; a refused gate
    /// never calls the readers after it.
    pub fn judge<E>(
        status: &str,
        read_counts: impl FnOnce() -> Result<CountsReading, E>,
        read_plan: impl FnOnce() -> Result<RoundPlan, E>,
    ) -> Result<Self, E> {
        if !CONSULTABLE_STATUSES.contains(&status) {
            return Ok(Self::Refused(Refusal::Status {
                status: status.to_owned(),
            }));
        }
        let reading = read_counts()?;
        let freshness = reading.freshness();
        if !freshness.is_fresh() {
            return Ok(Self::Refused(Refusal::Freshness {
                mismatches: freshness.mismatches,
            }));
        }
        if let Some(refusal) = Refusal::conducted(&read_plan()?) {
            return Ok(Self::Refused(refusal));
        }
        Ok(Self::Open(Documents::of(&reading)))
    }
}

impl Documents {
    fn of(reading: &CountsReading) -> Self {
        let under = |directory: &str, names: &[String]| {
            names
                .iter()
                .map(|name| format!("{directory}/{name}"))
                .collect()
        };
        Self {
            findings: under("findings", reading.findings.names()),
            reports: under("reports", reading.reports.names()),
        }
    }
}

impl Refusal {
    fn conducted(plan: &RoundPlan) -> Option<Self> {
        let unanswered: Vec<Pair> =
            plan.pairs.iter().map(|pair| pair.pair.clone()).collect();
        let held_back: Vec<HeldBackItem> = plan
            .items
            .iter()
            .filter(|item| !item.complete)
            .filter(|item| {
                !unanswered.iter().any(|pair| pair.question == item.question)
            })
            .map(|item| HeldBackItem {
                question: item.question.clone(),
                skips: plan
                    .skipped
                    .iter()
                    .filter(|skip| skip.pair.question == item.question)
                    .cloned()
                    .collect(),
            })
            .collect();
        (!unanswered.is_empty() || !held_back.is_empty()).then_some(
            Self::Conducted {
                unanswered,
                held_back,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::Consultation;
    use super::Documents;
    use super::HeldBackItem;
    use super::Refusal;
    use crate::topic::claims::ClaimedIndexes;
    use crate::topic::counts::CountedFindings;
    use crate::topic::counts::CountedReports;
    use crate::topic::counts::CountsReading;
    use crate::topic::counts::ManifestCount;
    use crate::topic::counts::Mismatch;
    use crate::topic::counts::StoredCount;
    use crate::topic::counts::StoredCounts;
    use crate::topic::layout::stem::Stem;
    use crate::topic::outline::Pair;
    use crate::topic::plan::ItemStatus;
    use crate::topic::plan::OutstandingPair;
    use crate::topic::plan::RoundPlan;
    use crate::topic::plan::Skip;
    use crate::topic::plan::SkipReason;
    use crate::topic::tree::Stage;
    use crate::topic::tree::WorstCaseTree;

    type Verdict = Result<Consultation, &'static str>;

    const QUESTION: &str = "How do agents plan?";

    fn reading(stored_findings: u32) -> CountsReading {
        let findings =
            CountedFindings::of(&["01-a-web.md", "02-b-web.md"], |_| {
                Ok::<_, Infallible>(Some(1))
            });
        let findings = match findings {
            Ok(findings) => findings,
            Err(never) => match never {},
        };
        CountsReading {
            stored: StoredCounts {
                round_count: StoredCount::Count(1),
                finding_count: StoredCount::Count(stored_findings),
                report_count: StoredCount::Missing,
                rounds_covered: StoredCount::Count(1),
            },
            findings,
            reports: CountedReports::of(&["a.md", "notes.txt"]),
        }
    }

    fn fresh() -> CountsReading {
        reading(2)
    }

    fn stale() -> CountsReading {
        reading(9)
    }

    fn plan(complete: bool) -> RoundPlan {
        RoundPlan {
            items: vec![ItemStatus {
                line: 3,
                question: QUESTION.to_owned(),
                complete,
            }],
            pairs: Vec::new(),
            skipped: Vec::new(),
            warnings: Vec::new(),
            shallow: Vec::new(),
            trims: Vec::new(),
            claims: ClaimedIndexes::default(),
        }
    }

    fn pair(profile: &str) -> Pair {
        Pair {
            question: QUESTION.to_owned(),
            profile: profile.to_owned(),
        }
    }

    fn skip(profile: &str) -> Skip {
        Skip {
            pair: pair(profile),
            reason: SkipReason::NotInstalled,
        }
    }

    fn with_unanswered_pair(mut plan: RoundPlan) -> RoundPlan {
        plan.pairs.push(OutstandingPair {
            pair: pair("openalex"),
            stem: Stem::parse("03-how-do-agents-plan-openalex")
                .unwrap_or_else(|| unreachable!()),
            stage: Stage::SinglePass,
            worst_case: WorstCaseTree::root(),
        });
        plan
    }

    fn judged(status: &str, counts: CountsReading, plan: RoundPlan) -> Verdict {
        Consultation::judge(status, || Ok(counts), || Ok(plan))
    }

    #[test]
    fn a_set_before_synthesis_is_refused_on_status() {
        for status in ["briefed", "outlined", "researching"] {
            assert_eq!(
                judged(status, fresh(), plan(true)),
                Ok(Consultation::Refused(Refusal::Status {
                    status: status.to_owned()
                })),
                "{status}"
            );
        }
    }

    #[test]
    fn a_synthesised_or_complete_set_passes_the_status_gate() {
        for status in ["synthesised", "complete"] {
            let verdict = judged(status, fresh(), plan(true));

            assert!(
                !matches!(
                    verdict,
                    Ok(Consultation::Refused(Refusal::Status { .. }))
                ),
                "{status}: {verdict:?}"
            );
        }
    }

    #[test]
    fn status_is_judged_before_freshness() {
        assert!(matches!(
            judged("researching", stale(), plan(true)),
            Ok(Consultation::Refused(Refusal::Status { .. }))
        ));
    }

    #[test]
    fn freshness_is_judged_before_the_outline() {
        assert_eq!(
            judged("synthesised", stale(), with_unanswered_pair(plan(false))),
            Ok(Consultation::Refused(Refusal::Freshness {
                mismatches: vec![Mismatch::Disagrees {
                    count: ManifestCount::FindingCount,
                    stored: 9,
                    derived: 2,
                }],
            }))
        );
    }

    #[test]
    fn an_unanswered_pair_holds_the_set_back() {
        assert_eq!(
            judged("synthesised", fresh(), with_unanswered_pair(plan(false))),
            Ok(Consultation::Refused(Refusal::Conducted {
                unanswered: vec![pair("openalex")],
                held_back: Vec::new(),
            }))
        );
    }

    #[test]
    fn an_item_with_only_skipped_pairs_holds_the_set_back() {
        let mut skipped = plan(false);
        skipped.skipped.push(skip("arxiv"));

        assert_eq!(
            judged("synthesised", fresh(), skipped),
            Ok(Consultation::Refused(Refusal::Conducted {
                unanswered: Vec::new(),
                held_back: vec![HeldBackItem {
                    question: QUESTION.to_owned(),
                    skips: vec![skip("arxiv")],
                }],
            }))
        );
    }

    #[test]
    fn a_skipped_pair_beside_an_answered_one_holds_nothing_back() {
        let mut answered = plan(true);
        answered.skipped.push(skip("arxiv"));

        assert!(matches!(
            judged("synthesised", fresh(), answered),
            Ok(Consultation::Open(_))
        ));
    }

    #[test]
    fn an_open_verdict_carries_the_counted_documents() {
        assert_eq!(
            judged("complete", fresh(), plan(true)),
            Ok(Consultation::Open(Documents {
                findings: vec![
                    "findings/01-a-web.md".to_owned(),
                    "findings/02-b-web.md".to_owned(),
                ],
                reports: vec!["reports/a.md".to_owned()],
            }))
        );
    }

    #[test]
    fn a_refused_gate_never_reads_the_next_gates_input() {
        let verdict: Verdict = Consultation::judge(
            "researching",
            || Err("counts read"),
            || Err("plan read"),
        );

        assert_eq!(
            verdict,
            Ok(Consultation::Refused(Refusal::Status {
                status: "researching".to_owned()
            }))
        );
    }

    #[test]
    fn a_failed_read_past_an_open_gate_propagates() {
        let counts_failed: Verdict = Consultation::judge(
            "synthesised",
            || Err("counts read"),
            || Ok(plan(true)),
        );
        let plan_failed: Verdict = Consultation::judge(
            "synthesised",
            || Ok(fresh()),
            || Err("plan read"),
        );

        assert_eq!(counts_failed, Err("counts read"));
        assert_eq!(plan_failed, Err("plan read"));
    }
}
