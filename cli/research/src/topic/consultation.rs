//! Whether `ask` and `report` may consult a set: three gates, applied in
//! order.

use crate::topic::counts::CountsReading;
use crate::topic::counts::Mismatch;
use crate::topic::layout::entry::finding_stem;
use crate::topic::layout::note_ref::NoteRef;
use crate::topic::layout::stem::Stem;
use crate::topic::layout::FINDINGS;
use crate::topic::layout::REPORTS;
use crate::topic::lifecycle::SetStatus;
use crate::topic::outline::Pair;
use crate::topic::plan::HoldBack;
use crate::topic::plan::RoundPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Consultation {
    Open(Documents),
    Refused(Refusal),
}

/// The set-relative paths a consultation reads, and the outline round a
/// proposal numbers from; only [`Consultation::judge`] builds them, so a set
/// is opened only through its gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Documents {
    findings: Vec<String>,
    reports: Vec<String>,
    composed: Vec<ComposedFinding>,
    highest_round: Option<u32>,
}

/// A finding composed from a `.levels/` tree, and that tree's notes in
/// lineage order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedFinding {
    pub finding: String,
    pub level_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    Status {
        status: SetStatus,
    },
    Freshness {
        mismatches: Vec<Mismatch>,
    },
    Conducted {
        unanswered: Vec<Pair>,
        held_back: Vec<HeldBackItem>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldBackItem {
    pub question: String,
    pub hold_back: HoldBack,
}

impl Consultation {
    /// # Errors
    ///
    /// Returns the error of the first reader that fails; a refused gate
    /// never calls the readers after it.
    pub fn judge<E>(
        status: SetStatus,
        read_counts: impl FnOnce() -> Result<CountsReading, E>,
        read_plan: impl FnOnce() -> Result<RoundPlan, E>,
        read_level_notes: impl FnOnce(&[Stem]) -> Result<Vec<NoteRef>, E>,
    ) -> Result<Self, E> {
        if !status.is_consultable() {
            return Ok(Self::Refused(Refusal::Status { status }));
        }
        let reading = read_counts()?;
        let freshness = reading.freshness();
        if !freshness.is_fresh() {
            return Ok(Self::Refused(Refusal::Freshness {
                mismatches: freshness.mismatches,
            }));
        }
        let plan = read_plan()?;
        if let Some(refusal) = Refusal::conducted(&plan) {
            return Ok(Self::Refused(refusal));
        }
        let stems: Vec<Stem> = reading
            .findings
            .names()
            .iter()
            .filter_map(|name| finding_stem(name))
            .collect();
        let level_notes = read_level_notes(&stems)?;
        Ok(Self::Open(Documents::of(
            &reading,
            level_notes,
            plan.highest_round,
        )))
    }
}

impl Documents {
    #[must_use]
    pub fn findings(&self) -> &[String] {
        &self.findings
    }

    #[must_use]
    pub fn reports(&self) -> &[String] {
        &self.reports
    }

    #[must_use]
    pub fn composed(&self) -> &[ComposedFinding] {
        &self.composed
    }

    #[must_use]
    pub const fn highest_round(&self) -> Option<u32> {
        self.highest_round
    }

    fn of(
        reading: &CountsReading,
        mut level_notes: Vec<NoteRef>,
        highest_round: Option<u32>,
    ) -> Self {
        let under =
            |directory: &str, name: &String| format!("{directory}/{name}");
        level_notes.sort();
        let composed = reading
            .findings
            .names()
            .iter()
            .filter_map(|name| {
                let stem = finding_stem(name)?;
                let level_notes: Vec<String> = level_notes
                    .iter()
                    .filter(|note| note.stem == stem)
                    .map(NoteRef::path)
                    .collect();
                (!level_notes.is_empty()).then(|| ComposedFinding {
                    finding: under(FINDINGS, name),
                    level_notes,
                })
            })
            .collect();
        Self {
            findings: reading
                .findings
                .names()
                .iter()
                .map(|name| under(FINDINGS, name))
                .collect(),
            reports: reading
                .reports
                .names()
                .iter()
                .map(|name| under(REPORTS, name))
                .collect(),
            composed,
            highest_round,
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
            .filter_map(|item| {
                Some(HeldBackItem {
                    question: item.question.clone(),
                    hold_back: item.held_back.clone()?,
                })
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
    use std::cell::RefCell;
    use std::convert::Infallible;

    use super::ComposedFinding;
    use super::Consultation;
    use super::Documents;
    use super::HeldBackItem;
    use super::Refusal;
    use crate::topic::claims::ClaimedIndexes;
    use crate::topic::counts::CountedFindings;
    use crate::topic::counts::CountedReports;
    use crate::topic::counts::CountsReading;
    use crate::topic::counts::FreshnessCount;
    use crate::topic::counts::Mismatch;
    use crate::topic::counts::StoredCount;
    use crate::topic::counts::StoredCounts;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::layout::note_ref::NoteRef;
    use crate::topic::layout::stem::Stem;
    use crate::topic::lifecycle::SetStatus;
    use crate::topic::outline::Pair;
    use crate::topic::plan::HoldBack;
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
                held_back: None,
            }],
            pairs: Vec::new(),
            skipped: Vec::new(),
            warnings: Vec::new(),
            shallow: Vec::new(),
            trims: Vec::new(),
            claims: ClaimedIndexes::default(),
            highest_round: Some(1),
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

    fn judged(
        status: SetStatus,
        counts: CountsReading,
        plan: RoundPlan,
    ) -> Verdict {
        Consultation::judge(
            status,
            || Ok(counts),
            || Ok(plan),
            |_| Ok(Vec::new()),
        )
    }

    fn note(stem: &str, lineage: &str) -> NoteRef {
        NoteRef {
            stem: Stem::parse(stem).unwrap_or_else(|| unreachable!()),
            lineage: Lineage::parse(lineage).unwrap_or_else(|| unreachable!()),
        }
    }

    #[test]
    fn a_set_before_synthesis_is_refused_on_status() {
        for status in [
            SetStatus::Briefed,
            SetStatus::Outlined,
            SetStatus::Researching,
        ] {
            assert_eq!(
                judged(status, fresh(), plan(true)),
                Ok(Consultation::Refused(Refusal::Status { status })),
                "{status:?}"
            );
        }
    }

    #[test]
    fn a_synthesised_or_complete_set_passes_the_status_gate() {
        for status in [SetStatus::Synthesised, SetStatus::Complete] {
            let verdict = judged(status, fresh(), plan(true));

            assert!(
                !matches!(
                    verdict,
                    Ok(Consultation::Refused(Refusal::Status { .. }))
                ),
                "{status:?}: {verdict:?}"
            );
        }
    }

    #[test]
    fn status_is_judged_before_freshness() {
        assert!(matches!(
            judged(SetStatus::Researching, stale(), plan(true)),
            Ok(Consultation::Refused(Refusal::Status { .. }))
        ));
    }

    #[test]
    fn freshness_is_judged_before_the_outline() {
        assert_eq!(
            judged(
                SetStatus::Synthesised,
                stale(),
                with_unanswered_pair(plan(false))
            ),
            Ok(Consultation::Refused(Refusal::Freshness {
                mismatches: vec![Mismatch::Disagrees {
                    count: FreshnessCount::FindingCount,
                    stored: 9,
                    derived: 2,
                }],
            }))
        );
    }

    #[test]
    fn an_unanswered_pair_holds_the_set_back() {
        assert_eq!(
            judged(
                SetStatus::Synthesised,
                fresh(),
                with_unanswered_pair(plan(false))
            ),
            Ok(Consultation::Refused(Refusal::Conducted {
                unanswered: vec![pair("openalex")],
                held_back: Vec::new(),
            }))
        );
    }

    fn held_back_by(hold_back: HoldBack) -> RoundPlan {
        let mut plan = plan(false);
        plan.items[0].held_back = Some(hold_back);
        plan
    }

    #[test]
    fn an_item_held_back_by_its_skips_holds_the_set_back() {
        let skips = HoldBack::Skipped(vec![skip("arxiv")]);

        assert_eq!(
            judged(
                SetStatus::Synthesised,
                fresh(),
                held_back_by(skips.clone())
            ),
            Ok(Consultation::Refused(Refusal::Conducted {
                unanswered: Vec::new(),
                held_back: vec![HeldBackItem {
                    question: QUESTION.to_owned(),
                    hold_back: skips,
                }],
            }))
        );
    }

    #[test]
    fn an_item_left_without_an_index_holds_the_set_back() {
        assert_eq!(
            judged(
                SetStatus::Synthesised,
                fresh(),
                held_back_by(HoldBack::IndexesExhausted)
            ),
            Ok(Consultation::Refused(Refusal::Conducted {
                unanswered: Vec::new(),
                held_back: vec![HeldBackItem {
                    question: QUESTION.to_owned(),
                    hold_back: HoldBack::IndexesExhausted,
                }],
            }))
        );
    }

    #[test]
    fn a_skipped_pair_beside_an_answered_one_holds_nothing_back() {
        let mut answered = plan(true);
        answered.skipped.push(skip("arxiv"));

        assert!(matches!(
            judged(SetStatus::Synthesised, fresh(), answered),
            Ok(Consultation::Open(_))
        ));
    }

    #[test]
    fn an_open_verdict_carries_the_counted_documents() {
        assert_eq!(
            judged(SetStatus::Complete, fresh(), plan(true)),
            Ok(Consultation::Open(Documents {
                findings: vec![
                    "findings/01-a-web.md".to_owned(),
                    "findings/02-b-web.md".to_owned(),
                ],
                reports: vec!["reports/a.md".to_owned()],
                composed: Vec::new(),
                highest_round: Some(1),
            }))
        );
    }

    #[test]
    fn an_open_verdict_lists_each_findings_level_notes_in_lineage_order() {
        let read_stems = RefCell::new(Vec::new());
        let verdict: Verdict = Consultation::judge(
            SetStatus::Complete,
            || Ok(fresh()),
            || Ok(plan(true)),
            |stems: &[Stem]| {
                read_stems.replace(stems.to_vec());
                Ok(vec![
                    note("02-b-web", "2-10"),
                    note("02-b-web", "1"),
                    note("02-b-web", "2-2"),
                ])
            },
        );

        let composed = match &verdict {
            Ok(Consultation::Open(documents)) => documents.composed().to_vec(),
            _ => Vec::new(),
        };
        assert_eq!(
            read_stems.into_inner(),
            [note("01-a-web", "1").stem, note("02-b-web", "1").stem]
        );
        assert_eq!(
            composed,
            [ComposedFinding {
                finding: "findings/02-b-web.md".to_owned(),
                level_notes: vec![
                    "findings/02-b-web.levels/1.md".to_owned(),
                    "findings/02-b-web.levels/2-2.md".to_owned(),
                    "findings/02-b-web.levels/2-10.md".to_owned(),
                ],
            }]
        );
    }

    #[test]
    fn a_refused_set_never_reads_its_level_notes() {
        let verdict: Verdict = Consultation::judge(
            SetStatus::Synthesised,
            || Ok(fresh()),
            || Ok(with_unanswered_pair(plan(false))),
            |_| Err("level notes read"),
        );

        assert!(matches!(
            verdict,
            Ok(Consultation::Refused(Refusal::Conducted { .. }))
        ));
    }

    #[test]
    fn a_failed_level_note_read_propagates() {
        let verdict: Verdict = Consultation::judge(
            SetStatus::Complete,
            || Ok(fresh()),
            || Ok(plan(true)),
            |_| Err("level notes read"),
        );

        assert_eq!(verdict, Err("level notes read"));
    }

    #[test]
    fn a_refused_gate_never_reads_the_next_gates_input() {
        let verdict: Verdict = Consultation::judge(
            SetStatus::Researching,
            || Err("counts read"),
            || Err("plan read"),
            |_| Err("level notes read"),
        );

        assert_eq!(
            verdict,
            Ok(Consultation::Refused(Refusal::Status {
                status: SetStatus::Researching
            }))
        );
    }

    #[test]
    fn a_failed_read_past_an_open_gate_propagates() {
        let counts_failed: Verdict = Consultation::judge(
            SetStatus::Synthesised,
            || Err("counts read"),
            || Ok(plan(true)),
            |_| Ok(Vec::new()),
        );
        let plan_failed: Verdict = Consultation::judge(
            SetStatus::Synthesised,
            || Ok(fresh()),
            || Err("plan read"),
            |_| Ok(Vec::new()),
        );

        assert_eq!(counts_failed, Err("counts read"));
        assert_eq!(plan_failed, Err("plan read"));
    }
}
