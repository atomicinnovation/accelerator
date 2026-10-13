//! What the run whose ledger a starting run replaced left of the batch it
//! last offered: each spawn classed by the one file it writes.

use crate::conduct::ledger::RunId;
use crate::conduct::ledger::RunLedger;
use crate::conduct::spawn::SpawnRef;
use crate::topic::claims::ClaimedIndexes;
use crate::topic::evidence::Finding;
use crate::topic::evidence::LevelsDirectory;
use crate::topic::layout::note_ref::NoteRef;
use crate::topic::layout::stem::Stem;
use crate::topic::outline::Outline;
use crate::topic::outline::OutlineItem;
use crate::topic::outline::Pair;
use crate::topic::plan::RoundInputs;
use crate::topic::plan::RoundPlan;
use crate::topic::question::NormalisedQuestion;
use crate::topic::question::UnicodeText;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacedBatch {
    run: RunId,
    quarantine: Vec<Stem>,
    spawns: Vec<Pending>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    Judged(Verdict),
    AwaitingPlan { at: NoteRef, owner: Option<Pair> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Verdict {
    leftover: Leftover,
    outcome: SpawnOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacedRun {
    run: RunId,
    valid: Vec<Leftover>,
    invalid: Vec<Leftover>,
    unwritten: Vec<Leftover>,
    unasked: Vec<Leftover>,
    quarantine: Vec<Stem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leftover {
    spawn: SpawnRef,
    pair: Option<Pair>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpawnOutcome {
    Valid,
    Invalid,
    Unwritten,
    Unasked,
}

impl ReplacedBatch {
    #[must_use]
    pub fn judging_findings(
        replaced: &RunLedger,
        inputs: &RoundInputs,
        unicode: &dyn UnicodeText,
    ) -> Self {
        let judge = Judge {
            outline: &inputs.outline,
            claims: replaced.claims(),
            unicode,
        };
        let mut quarantine = Vec::new();
        let spawns = replaced
            .memory()
            .pending
            .spawns()
            .iter()
            .map(|spawn| match spawn {
                SpawnRef::Pair(stem) => {
                    let verdict = judge.finding(stem, inputs);
                    if verdict.outcome == SpawnOutcome::Invalid {
                        quarantine.push(stem.clone());
                    }
                    Pending::Judged(verdict)
                }
                SpawnRef::Node(at) => Pending::AwaitingPlan {
                    at: at.clone(),
                    owner: judge.owner(&at.stem),
                },
            })
            .collect();
        Self {
            run: replaced.run().clone(),
            quarantine,
            spawns,
        }
    }

    #[must_use]
    pub fn quarantining(
        &self,
        inputs: RoundInputs,
        unicode: &dyn UnicodeText,
    ) -> RoundInputs {
        inputs.with_findings_quarantined(&self.quarantine, unicode)
    }

    #[must_use]
    pub fn report(
        self,
        inputs: &RoundInputs,
        plan: &RoundPlan,
        unicode: &dyn UnicodeText,
    ) -> ReplacedRun {
        let mut report = ReplacedRun {
            run: self.run,
            valid: Vec::new(),
            invalid: Vec::new(),
            unwritten: Vec::new(),
            unasked: Vec::new(),
            quarantine: self.quarantine,
        };
        for pending in self.spawns {
            let verdict = match pending {
                Pending::Judged(verdict) => verdict,
                Pending::AwaitingPlan { at, owner } => {
                    let outcome = note_outcome(
                        &at,
                        owner.as_ref(),
                        inputs,
                        plan,
                        unicode,
                    );
                    Verdict {
                        leftover: Leftover {
                            spawn: SpawnRef::Node(at),
                            pair: owner,
                        },
                        outcome,
                    }
                }
            };
            report.class_of(verdict.outcome).push(verdict.leftover);
        }
        report
    }
}

impl ReplacedRun {
    #[must_use]
    pub const fn run(&self) -> &RunId {
        &self.run
    }

    #[must_use]
    pub fn valid(&self) -> &[Leftover] {
        &self.valid
    }

    #[must_use]
    pub fn invalid(&self) -> &[Leftover] {
        &self.invalid
    }

    #[must_use]
    pub fn unwritten(&self) -> &[Leftover] {
        &self.unwritten
    }

    #[must_use]
    pub fn unasked(&self) -> &[Leftover] {
        &self.unasked
    }

    #[must_use]
    pub fn quarantine(&self) -> &[Stem] {
        &self.quarantine
    }

    const fn class_of(&mut self, outcome: SpawnOutcome) -> &mut Vec<Leftover> {
        match outcome {
            SpawnOutcome::Valid => &mut self.valid,
            SpawnOutcome::Invalid => &mut self.invalid,
            SpawnOutcome::Unwritten => &mut self.unwritten,
            SpawnOutcome::Unasked => &mut self.unasked,
        }
    }
}

impl Leftover {
    #[must_use]
    pub const fn spawn(&self) -> &SpawnRef {
        &self.spawn
    }

    #[must_use]
    pub const fn pair(&self) -> Option<&Pair> {
        self.pair.as_ref()
    }
}

struct Judge<'a> {
    outline: &'a Outline,
    claims: &'a ClaimedIndexes,
    unicode: &'a dyn UnicodeText,
}

impl Judge<'_> {
    fn owner(&self, stem: &Stem) -> Option<Pair> {
        let claimants: Vec<&OutlineItem> = self
            .outline
            .items
            .iter()
            .filter(|item| {
                let question =
                    NormalisedQuestion::of(&item.question, self.unicode);
                self.claims.index_for(&question) == Some(stem.index())
            })
            .collect();
        claimants
            .iter()
            .find_map(|item| item.pair_at(stem))
            .or_else(|| {
                claimants.iter().find_map(|item| item.pair_ending(stem))
            })
    }

    fn finding(&self, stem: &Stem, inputs: &RoundInputs) -> Verdict {
        let finding = inputs
            .findings
            .iter()
            .find(|finding| finding.stem() == Some(stem));
        let (outcome, pair) = self.owner(stem).map_or_else(
            || self.unowned_finding(finding),
            |owner| (self.owned_finding(finding, &owner), Some(owner)),
        );
        Verdict {
            leftover: Leftover {
                spawn: SpawnRef::Pair(stem.clone()),
                pair,
            },
            outcome,
        }
    }

    fn owned_finding(
        &self,
        finding: Option<&Finding>,
        owner: &Pair,
    ) -> SpawnOutcome {
        let question = NormalisedQuestion::of(&owner.question, self.unicode);
        match finding {
            None => SpawnOutcome::Unwritten,
            Some(finding) if finding.answers(&question, &owner.profile) => {
                SpawnOutcome::Valid
            }
            Some(_) => SpawnOutcome::Invalid,
        }
    }

    fn unowned_finding(
        &self,
        finding: Option<&Finding>,
    ) -> (SpawnOutcome, Option<Pair>) {
        match finding.map(Finding::answer) {
            None => (SpawnOutcome::Unasked, None),
            Some(None) => (SpawnOutcome::Invalid, None),
            Some(Some(answer)) if self.asks(&answer.pair) => {
                (SpawnOutcome::Invalid, None)
            }
            Some(Some(answer)) => {
                (SpawnOutcome::Unasked, Some(answer.pair.clone()))
            }
        }
    }

    fn asks(&self, pair: &Pair) -> bool {
        let question = NormalisedQuestion::of(&pair.question, self.unicode);
        self.outline.items.iter().any(|item| {
            NormalisedQuestion::of(&item.question, self.unicode) == question
                && item.profiles().contains(&pair.profile.as_str())
        })
    }
}

fn note_outcome(
    at: &NoteRef,
    owner: Option<&Pair>,
    inputs: &RoundInputs,
    plan: &RoundPlan,
    unicode: &dyn UnicodeText,
) -> SpawnOutcome {
    let Some(owner) = owner else {
        return SpawnOutcome::Unasked;
    };
    let Some(directory) = inputs.levels.iter().find(|directory| {
        *directory.stem() == at.stem && directory.holds(&at.lineage)
    }) else {
        return SpawnOutcome::Unwritten;
    };
    if directory.refused(&at.lineage)
        || plan.rejects(at)
        || !grew_from(directory, owner, unicode)
    {
        SpawnOutcome::Invalid
    } else {
        SpawnOutcome::Valid
    }
}

fn grew_from(
    directory: &LevelsDirectory,
    owner: &Pair,
    unicode: &dyn UnicodeText,
) -> bool {
    directory.root_question().is_none_or(|root| {
        NormalisedQuestion::of(root, unicode)
            == NormalisedQuestion::of(&owner.question, unicode)
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeMap;

    use super::Leftover;
    use super::ReplacedBatch;
    use super::ReplacedRun;
    use crate::conduct::ledger::RunId;
    use crate::conduct::ledger::RunLedger;
    use crate::conduct::memory::PendingBatch;
    use crate::conduct::memory::RunMemory;
    use crate::conduct::spawn::SpawnRef;
    use crate::topic::claims::ClaimedIndexes;
    use crate::topic::evidence::Digest;
    use crate::topic::evidence::Finding;
    use crate::topic::evidence::LevelNote;
    use crate::topic::evidence::LevelsDirectory;
    use crate::topic::evidence::NoteRejection;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::layout::stem::Stem;
    use crate::topic::outline::Outline;
    use crate::topic::outline::Pair;
    use crate::topic::plan::RoundInputs;
    use crate::topic::plan::RoundPlan;
    use crate::topic::question::FakeUnicode;
    use crate::topic::question::NormalisedQuestion;
    use crate::topic::tree::Depth;

    fn pair(question: &str, profile: &str) -> Pair {
        Pair {
            question: question.to_owned(),
            profile: profile.to_owned(),
        }
    }

    fn stem(text: &str) -> Stem {
        Stem::parse(text).expect("a stem")
    }

    fn lineage(text: &str) -> Lineage {
        Lineage::parse(text).expect("a lineage")
    }

    fn inputs(lines: &[&str]) -> RoundInputs {
        RoundInputs {
            outline: Outline::parse(&lines.join("\n")),
            source_profiles: vec!["web".into(), "openalex".into()],
            available_profiles: vec!["web".into(), "openalex".into()],
            ..RoundInputs::default()
        }
    }

    fn at_depth(lines: &[&str], levels: u32) -> RoundInputs {
        RoundInputs {
            depth: Depth::new(levels).expect("a depth"),
            ..inputs(lines)
        }
    }

    fn answering(name: &str, question: &str, profile: &str) -> Finding {
        Finding::retained(name, pair(question, profile), &FakeUnicode)
    }

    fn note(at: &str, question: &str, follow_ups: &[&str]) -> LevelNote {
        LevelNote {
            lineage: lineage(at),
            question: question.to_owned(),
            follow_ups: follow_ups.iter().map(|&f| f.to_owned()).collect(),
            digest: Digest::new([3; 32]),
        }
    }

    fn tree(
        at: &str,
        root_question: Option<&str>,
        notes: Vec<LevelNote>,
        refused: &[&str],
    ) -> LevelsDirectory {
        LevelsDirectory::new(
            stem(at),
            root_question.map(str::to_owned),
            notes,
            refused
                .iter()
                .map(|at| (lineage(at), NoteRejection::WrongKind))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    fn replaced(claims: &[(&str, u32)], batch: &[&str]) -> RunLedger {
        let mut claimed = ClaimedIndexes::default();
        for (question, index) in claims {
            claimed
                .claim(NormalisedQuestion::of(question, &FakeUnicode), *index);
        }
        let spawns = batch
            .iter()
            .map(|text| SpawnRef::parse(text).expect("a spawn ref"))
            .collect();
        RunLedger::from_parts(
            RunId::parse("earlier").expect("a run id"),
            claimed,
            RunMemory {
                pending: PendingBatch::new(3, spawns),
                ..RunMemory::default()
            },
        )
    }

    fn report(
        inputs: RoundInputs,
        claims: &[(&str, u32)],
        batch: &[&str],
    ) -> ReplacedRun {
        let replaced = replaced(claims, batch);
        let judged =
            ReplacedBatch::judging_findings(&replaced, &inputs, &FakeUnicode);
        let inputs = judged.quarantining(inputs, &FakeUnicode);
        let plan = RoundPlan::of(&inputs, &FakeUnicode);
        judged.report(&inputs, &plan, &FakeUnicode)
    }

    fn spawns(leftovers: &[Leftover]) -> Vec<String> {
        leftovers
            .iter()
            .map(|leftover| leftover.spawn().to_string())
            .collect()
    }

    fn owners(leftovers: &[Leftover]) -> Vec<Option<Pair>> {
        leftovers
            .iter()
            .map(|leftover| leftover.pair().cloned())
            .collect()
    }

    fn quarantine(report: &ReplacedRun) -> Vec<String> {
        report
            .quarantine()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn a_pair_whose_finding_answers_its_owning_pair_is_valid() {
        let mut inputs = inputs(&["- [ ] A?"]);
        inputs.findings = vec![answering("01-a-web.md", "A?", "web")];
        let report = report(inputs, &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.valid()), ["01-a-web"]);
        assert!(report.invalid().is_empty());
        assert!(report.quarantine().is_empty());
    }

    #[test]
    fn a_pair_whose_finding_fails_validation_is_invalid() {
        let mut inputs = inputs(&["- [ ] A?"]);
        inputs.findings = vec![Finding::invalid("01-a-web.md")];
        let report = report(inputs, &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web"]);
        assert_eq!(owners(report.invalid()), [Some(pair("A?", "web"))]);
        assert_eq!(quarantine(&report), ["01-a-web"]);
    }

    #[test]
    fn a_pair_whose_finding_names_a_question_no_outline_pair_has_is_invalid() {
        let mut inputs = inputs(&["- [ ] A?"]);
        inputs.findings = vec![answering("01-a-web.md", "Z?", "web")];
        let report = report(inputs, &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web"]);
        assert_eq!(quarantine(&report), ["01-a-web"]);
    }

    #[test]
    fn a_pair_whose_finding_names_a_profile_its_item_lacks_is_invalid() {
        let mut inputs = inputs(&["- [ ] A?"]);
        inputs.findings = vec![answering("01-a-web.md", "A?", "openalex")];
        let report = report(inputs, &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web"]);
        assert_eq!(quarantine(&report), ["01-a-web"]);
    }

    #[test]
    fn a_pair_whose_finding_answers_another_outline_pair_is_invalid() {
        let mut inputs = inputs(&["- [ ] A?", "- [ ] B?"]);
        inputs.findings = vec![answering("02-b-web.md", "A?", "web")];
        let report =
            report(inputs, &[("A?", 1), ("B?", 2)], &["01-a-web", "02-b-web"]);
        assert_eq!(spawns(report.invalid()), ["02-b-web"]);
        assert_eq!(owners(report.invalid()), [Some(pair("B?", "web"))]);
        assert_eq!(spawns(report.unwritten()), ["01-a-web"]);
        assert_eq!(quarantine(&report), ["02-b-web"]);
    }

    #[test]
    fn a_pair_whose_finding_differs_from_its_question_only_in_normalisation_is_valid(
    ) {
        let mut inputs = inputs(&["- [ ] A   b?"]);
        inputs.findings =
            vec![answering("01-a-b-web.md", " A b\u{FF1F}", "web")];
        let report = report(inputs, &[("A b?", 1)], &["01-a-b-web"]);
        assert_eq!(spawns(report.valid()), ["01-a-b-web"]);
        assert_eq!(owners(report.valid()), [Some(pair("A   b?", "web"))]);
    }

    #[test]
    fn a_pair_whose_question_was_reworded_to_the_same_normalised_form_keeps_its_finding(
    ) {
        let mut inputs = inputs(&["- [ ] \u{FF21}?"]);
        inputs.findings = vec![answering("01-a-web.md", "A?", "web")];
        let report = report(inputs, &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.valid()), ["01-a-web"]);
        assert_eq!(owners(report.valid()), [Some(pair("\u{FF21}?", "web"))]);
        assert!(report.quarantine().is_empty());
    }

    #[test]
    fn a_pair_sharing_its_slug_with_another_is_judged_against_the_question_its_run_claimed(
    ) {
        let mut inputs = inputs(&["- [ ] 注意力？", "- [ ] 机制？"]);
        inputs.findings = vec![
            answering("01-focus-area-web.md", "注意力？", "web"),
            answering("02-focus-area-web.md", "注意力？", "web"),
        ];
        let report = report(
            inputs,
            &[("注意力？", 1), ("机制？", 2)],
            &["01-focus-area-web", "02-focus-area-web"],
        );
        assert_eq!(spawns(report.valid()), ["01-focus-area-web"]);
        assert_eq!(spawns(report.invalid()), ["02-focus-area-web"]);
        assert_eq!(owners(report.invalid()), [Some(pair("机制？", "web"))]);
    }

    #[test]
    fn a_pair_with_no_finding_is_unwritten() {
        let report = report(inputs(&["- [ ] A?"]), &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.unwritten()), ["01-a-web"]);
        assert_eq!(owners(report.unwritten()), [Some(pair("A?", "web"))]);
        assert!(report.quarantine().is_empty());
    }

    #[test]
    fn a_valid_finding_whose_question_left_the_outline_is_unasked_with_the_pair_it_names(
    ) {
        let mut inputs = inputs(&["- [ ] A2?"]);
        inputs.findings = vec![answering("01-a-web.md", "A?", "web")];
        let report = report(inputs, &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.unasked()), ["01-a-web"]);
        assert_eq!(owners(report.unasked()), [Some(pair("A?", "web"))]);
        assert!(report.quarantine().is_empty());
    }

    #[test]
    fn a_valid_finding_whose_profile_left_its_item_is_unasked() {
        let mut inputs = inputs(&["- [ ] A? — profiles: openalex"]);
        inputs.findings = vec![answering("01-a-web.md", "A?", "web")];
        let report = report(inputs, &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.unasked()), ["01-a-web"]);
        assert_eq!(owners(report.unasked()), [Some(pair("A?", "web"))]);
        assert!(report.quarantine().is_empty());
    }

    #[test]
    fn a_finding_at_an_unowned_stem_answering_an_outline_pair_is_invalid() {
        let mut inputs = inputs(&["- [ ] A2?", "- [ ] B?"]);
        inputs.findings = vec![answering("01-a-web.md", "B?", "web")];
        let report = report(inputs, &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web"]);
        assert_eq!(owners(report.invalid()), [None]);
        assert_eq!(quarantine(&report), ["01-a-web"]);
    }

    #[test]
    fn a_finding_failing_validation_at_an_unowned_stem_is_invalid() {
        let mut inputs = inputs(&["- [ ] A2?"]);
        inputs.findings = vec![Finding::invalid("01-a-web.md")];
        let report = report(inputs, &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web"]);
        assert_eq!(owners(report.invalid()), [None]);
        assert_eq!(quarantine(&report), ["01-a-web"]);
    }

    #[test]
    fn an_unwritten_spawn_whose_question_left_the_outline_is_unasked() {
        let report =
            report(inputs(&["- [ ] A2?"]), &[("A?", 1)], &["01-a-web"]);
        assert_eq!(spawns(report.unasked()), ["01-a-web"]);
        assert_eq!(owners(report.unasked()), [None]);
    }

    #[test]
    fn a_node_whose_note_the_plan_accepts_is_valid() {
        let mut inputs = at_depth(&["- [ ] A?"], 2);
        inputs.levels = vec![tree(
            "01-a-web",
            Some("A?"),
            vec![note("1", "A?", &["B?"])],
            &[],
        )];
        let report = report(inputs, &[("A?", 1)], &["01-a-web:1"]);
        assert_eq!(spawns(report.valid()), ["01-a-web:1"]);
        assert_eq!(owners(report.valid()), [Some(pair("A?", "web"))]);
    }

    #[test]
    fn a_node_whose_note_was_refused_is_invalid() {
        let mut inputs = at_depth(&["- [ ] A?"], 2);
        inputs.levels = vec![tree("01-a-web", Some("A?"), Vec::new(), &["1"])];
        let report = report(inputs, &[("A?", 1)], &["01-a-web:1"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web:1"]);
        assert!(report.quarantine().is_empty());
    }

    #[test]
    fn a_node_whose_note_the_plan_rejects_against_its_candidate_is_invalid() {
        let mut inputs = at_depth(&["- [ ] A?"], 2);
        inputs.levels = vec![tree(
            "01-a-web",
            Some("A?"),
            vec![note("1", "A?", &["B?"]), note("2-1", "X?", &[])],
            &[],
        )];
        let report = report(inputs, &[("A?", 1)], &["01-a-web:2-1"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web:2-1"]);
    }

    #[test]
    fn a_node_with_no_note_is_unwritten() {
        let mut inputs = at_depth(&["- [ ] A?", "- [ ] B?"], 2);
        inputs.levels = vec![tree(
            "01-a-web",
            Some("A?"),
            vec![note("1", "A?", &["X?"])],
            &[],
        )];
        let report = report(
            inputs,
            &[("A?", 1), ("B?", 2)],
            &["01-a-web:2-1", "02-b-web:1"],
        );
        assert_eq!(spawns(report.unwritten()), ["01-a-web:2-1", "02-b-web:1"]);
    }

    #[test]
    fn a_node_no_longer_planned_is_classed_by_its_note() {
        let mut inputs = at_depth(&["- [ ] A?"], 2);
        inputs.levels = vec![tree(
            "01-a-web",
            Some("A?"),
            vec![note("1", "A?", &["B?"]), note("2-1", "B?", &[])],
            &[],
        )];
        let report = report(inputs, &[("A?", 1)], &["01-a-web:2-1"]);
        assert_eq!(spawns(report.valid()), ["01-a-web:2-1"]);
    }

    #[test]
    fn a_node_whose_tree_grew_from_another_question_is_invalid() {
        let mut inputs = at_depth(&["- [ ] A?"], 2);
        inputs.levels = vec![tree(
            "01-a-web",
            Some("Other?"),
            vec![note("1", "Other?", &[])],
            &[],
        )];
        let report = report(inputs, &[("A?", 1)], &["01-a-web:1"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web:1"]);
        assert_eq!(owners(report.invalid()), [Some(pair("A?", "web"))]);
    }

    #[test]
    fn a_deeper_node_in_an_orphaned_tree_is_invalid() {
        let mut inputs = at_depth(&["- [ ] A?"], 2);
        inputs.levels = vec![tree(
            "01-a-web",
            Some("Other?"),
            vec![note("1", "Other?", &["X?"]), note("2-1", "X?", &[])],
            &[],
        )];
        let report = report(inputs, &[("A?", 1)], &["01-a-web:2-1"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web:2-1"]);
    }

    #[test]
    fn a_lineage_with_no_note_in_an_orphaned_tree_is_unwritten() {
        let mut inputs = at_depth(&["- [ ] A?", "- [ ] B?"], 2);
        inputs.levels = vec![
            tree(
                "01-a-web",
                Some("Other?"),
                vec![note("1", "Other?", &["X?"])],
                &[],
            ),
            tree("02-b-web", Some("Elsewhere?"), Vec::new(), &[]),
        ];
        let report = report(
            inputs,
            &[("A?", 1), ("B?", 2)],
            &["01-a-web:2-1", "02-b-web:1"],
        );
        assert_eq!(spawns(report.unwritten()), ["01-a-web:2-1", "02-b-web:1"]);
    }

    #[test]
    fn a_node_whose_tree_root_matches_only_after_normalisation_is_valid() {
        let mut inputs = at_depth(&["- [ ] A?"], 2);
        inputs.levels = vec![tree(
            "01-a-web",
            Some(" A\u{FF1F}"),
            vec![note("1", " A\u{FF1F}", &["B?"])],
            &[],
        )];
        let report = report(inputs, &[("A?", 1)], &["01-a-web:1"]);
        assert_eq!(spawns(report.valid()), ["01-a-web:1"]);
    }

    #[test]
    fn a_node_in_a_tree_with_no_root_question_is_classed_by_its_note() {
        let mut inputs = at_depth(&["- [ ] A?"], 2);
        inputs.levels =
            vec![tree("01-a-web", None, vec![note("2-1", "B?", &[])], &["1"])];
        let report =
            report(inputs, &[("A?", 1)], &["01-a-web:1", "01-a-web:2-1"]);
        assert_eq!(spawns(report.invalid()), ["01-a-web:1"]);
        assert_eq!(spawns(report.valid()), ["01-a-web:2-1"]);
    }

    #[test]
    fn a_node_whose_question_left_the_outline_is_unasked_whatever_its_note() {
        let mut inputs = at_depth(&["- [ ] B?"], 2);
        inputs.levels = vec![tree(
            "01-a-web",
            Some("A?"),
            vec![note("1", "A?", &["X?", "Y?"]), note("2-1", "X?", &[])],
            &["2-2"],
        )];
        let batch = [
            "01-a-web:1",
            "01-a-web:2-1",
            "01-a-web:2-2",
            "01-a-web:3-1-1",
        ];
        let report = report(inputs, &[("A?", 1)], &batch);
        assert_eq!(spawns(report.unasked()), batch);
        assert_eq!(owners(report.unasked()), [None, None, None, None]);
        assert!(report.invalid().is_empty());
    }

    #[test]
    fn a_misdirected_finding_does_not_hide_the_rejections_of_the_pair_it_names()
    {
        let mut inputs = at_depth(&["- [ ] A?", "- [ ] B?"], 2);
        inputs.findings = vec![answering("02-b-web.md", "A?", "web")];
        inputs.levels = vec![tree(
            "01-a-web",
            Some("A?"),
            vec![note("1", "A?", &["X?"]), note("2-1", "Y?", &[])],
            &[],
        )];
        let report = report(
            inputs,
            &[("A?", 1), ("B?", 2)],
            &["01-a-web:2-1", "02-b-web"],
        );
        assert_eq!(spawns(report.invalid()), ["01-a-web:2-1", "02-b-web"]);
    }

    #[test]
    fn each_leftover_carries_its_owning_pair() {
        let mut inputs =
            inputs(&["- [ ] A? — profiles: web, openalex", "- [ ] B?"]);
        inputs.findings = vec![answering("01-a-openalex.md", "A?", "openalex")];
        let report = report(
            inputs,
            &[("A?", 1), ("B?", 2)],
            &["01-a-web", "01-a-openalex", "02-b-web"],
        );
        assert_eq!(owners(report.valid()), [Some(pair("A?", "openalex"))]);
        assert_eq!(
            owners(report.unwritten()),
            [Some(pair("A?", "web")), Some(pair("B?", "web"))]
        );
    }

    #[test]
    fn each_class_keeps_the_order_of_the_last_batch() {
        let mut inputs = at_depth(&["- [ ] A?", "- [ ] B?", "- [ ] C?"], 1);
        inputs.findings = vec![
            answering("01-a-web.md", "A?", "web"),
            answering("03-c-web.md", "C?", "web"),
        ];
        let report = report(
            inputs,
            &[("A?", 1), ("B?", 2), ("C?", 3), ("D?", 4)],
            &["03-c-web", "04-d-web", "01-a-web", "02-b-web:1"],
        );
        assert_eq!(spawns(report.valid()), ["03-c-web", "01-a-web"]);
        assert_eq!(spawns(report.unasked()), ["04-d-web"]);
        assert_eq!(spawns(report.unwritten()), ["02-b-web:1"]);
    }

    #[test]
    fn an_empty_last_batch_names_its_run_and_classes_nothing() {
        let report = report(inputs(&["- [ ] A?"]), &[], &[]);
        assert_eq!(report.run().to_string(), "earlier");
        assert!(report.valid().is_empty());
        assert!(report.invalid().is_empty());
        assert!(report.unwritten().is_empty());
        assert!(report.unasked().is_empty());
        assert!(report.quarantine().is_empty());
    }

    #[test]
    fn only_invalid_findings_are_quarantined() {
        let mut inputs = at_depth(&["- [ ] A?", "- [ ] B?"], 2);
        inputs.findings = vec![
            Finding::invalid("02-b-web.md"),
            answering("03-z-web.md", "Z?", "web"),
        ];
        inputs.levels = vec![tree("01-a-web", Some("A?"), Vec::new(), &["1"])];
        let report = report(
            inputs,
            &[("A?", 1), ("B?", 2), ("Z?", 3)],
            &["01-a-web:1", "03-z-web", "02-b-web"],
        );
        assert_eq!(spawns(report.invalid()), ["01-a-web:1", "02-b-web"]);
        assert_eq!(spawns(report.unasked()), ["03-z-web"]);
        assert_eq!(quarantine(&report), ["02-b-web"]);
    }

    #[test]
    fn the_report_quarantines_what_the_batch_read_as_quarantined() {
        let mut inputs = inputs(&["- [ ] A?", "- [ ] B?", "- [ ] C?"]);
        inputs.findings = vec![
            Finding::invalid("01-a-web.md"),
            answering("02-b-web.md", "B?", "web"),
            answering("03-c-web.md", "Z?", "web"),
        ];
        let replaced = replaced(
            &[("A?", 1), ("B?", 2), ("C?", 3)],
            &["01-a-web", "02-b-web", "03-c-web"],
        );
        let judged =
            ReplacedBatch::judging_findings(&replaced, &inputs, &FakeUnicode);
        let quarantined = judged.quarantining(inputs, &FakeUnicode);
        let set_aside: Vec<Stem> = ["01-a-web", "03-c-web"].map(stem).into();
        assert!(quarantined.findings.iter().all(|finding| !finding
            .stem()
            .is_some_and(|at| set_aside.contains(at))));
        assert_eq!(quarantined.findings.len(), 1);
        let plan = RoundPlan::of(&quarantined, &FakeUnicode);
        let report = judged.report(&quarantined, &plan, &FakeUnicode);
        assert_eq!(report.quarantine(), set_aside);
    }
}
