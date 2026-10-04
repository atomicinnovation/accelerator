//! Planning a `conduct` round: which pairs an outline asks for, which of them
//! a retained finding already answers, and what each outstanding pair needs
//! next.

use std::collections::BTreeMap;

use crate::topic::claims::ClaimedIndexes;
use crate::topic::claims::IndexClaim;
use crate::topic::evidence::Finding;
use crate::topic::evidence::LevelsDirectory;
use crate::topic::layout::slug::QuestionSlug;
use crate::topic::layout::stem::Stem;
use crate::topic::outline::Outline;
use crate::topic::outline::OutlineItem;
use crate::topic::outline::Pair;
use crate::topic::question::NormalisedQuestion;
use crate::topic::question::UnicodeText;
use crate::topic::tree::derive;
use crate::topic::tree::Depth;
use crate::topic::tree::Stage;
use crate::topic::tree::Trim;

/// Everything a round plan is derived from, read from one set.
#[derive(Debug, Clone, Default)]
pub struct RoundInputs {
    pub outline: Outline,
    pub findings: Vec<Finding>,
    pub quarantined: Vec<IndexClaim>,
    /// The brief's `source_profiles`.
    pub source_profiles: Vec<String>,
    /// The profiles with an installed `<name>-profile` skill.
    pub available_profiles: Vec<String>,
    pub levels: Vec<LevelsDirectory>,
    pub depth: Depth,
    /// The indexes the run planning this round has already claimed.
    pub claims: ClaimedIndexes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundPlan {
    pub items: Vec<ItemStatus>,
    pub pairs: Vec<OutstandingPair>,
    pub skipped: Vec<Skip>,
    pub warnings: Vec<Warning>,
    pub shallow: Vec<ShallowFinding>,
    pub trims: Vec<Trim>,
    /// The index each focus area with an outstanding pair was planned at.
    pub claims: ClaimedIndexes,
}

/// Whether an outline item is complete: it has at least one eligible pair,
/// and a retained finding answers every one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStatus {
    pub line: usize,
    pub question: String,
    pub complete: bool,
}

/// A pair no retained finding answers yet, under the stem its finding and
/// level notes are named by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutstandingPair {
    pub pair: Pair,
    pub stem: Stem,
    pub stage: Stage,
}

/// A retained finding researched to less than the requested depth, which
/// `conduct` does not deepen again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShallowFinding {
    pub stem: Stem,
    pub depth: Depth,
}

/// A pair `conduct` cannot research.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skip {
    pub pair: Pair,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    NotInBrief,
    NotInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    UnseparatedProfiles { line: usize },
    UnmatchedFinding { name: String, question: String },
    IndexesExhausted { question: String },
}

impl RoundPlan {
    #[must_use]
    pub fn of(inputs: &RoundInputs, unicode: &dyn UnicodeText) -> Self {
        Planner::new(inputs, unicode).plan()
    }
}

struct Planner<'a> {
    inputs: &'a RoundInputs,
    unicode: &'a dyn UnicodeText,
    file_claims: Vec<IndexClaim>,
    next_unclaimed: Option<u32>,
    claimed: BTreeMap<NormalisedQuestion, Option<u32>>,
}

impl<'a> Planner<'a> {
    fn new(inputs: &'a RoundInputs, unicode: &'a dyn UnicodeText) -> Self {
        let file_claims: Vec<IndexClaim> = inputs
            .quarantined
            .iter()
            .cloned()
            .chain(inputs.levels.iter().map(|levels| levels.claim(unicode)))
            .collect();
        let highest = inputs
            .findings
            .iter()
            .filter_map(Finding::index)
            .chain(file_claims.iter().map(IndexClaim::index))
            .chain(inputs.claims.indexes())
            .max();
        Self {
            inputs,
            unicode,
            file_claims,
            next_unclaimed: highest
                .map_or(Some(1), |index| index.checked_add(1)),
            claimed: BTreeMap::new(),
        }
    }

    fn normalised(&self, question: &str) -> NormalisedQuestion {
        NormalisedQuestion::of(question, self.unicode)
    }

    fn plan(mut self) -> RoundPlan {
        let mut plan = RoundPlan {
            items: Vec::new(),
            pairs: Vec::new(),
            skipped: Vec::new(),
            warnings: Vec::new(),
            shallow: Vec::new(),
            trims: Vec::new(),
            claims: ClaimedIndexes::default(),
        };
        for item in &self.inputs.outline.items {
            if item.names_profiles_unseparated() {
                plan.warnings
                    .push(Warning::UnseparatedProfiles { line: item.line });
            }
            let complete = self.plan_item(item, &mut plan);
            plan.items.push(ItemStatus {
                line: item.line,
                question: item.question.clone(),
                complete,
            });
        }
        plan.warnings.extend(self.unmatched_findings());
        for (question, index) in &self.claimed {
            if let Some(index) = index {
                plan.claims.claim(question.clone(), *index);
            }
        }
        plan
    }

    fn plan_item(&mut self, item: &OutlineItem, plan: &mut RoundPlan) -> bool {
        let question = self.normalised(&item.question);
        let first_sighting = !self.claimed.contains_key(&question);
        let mut eligible = 0;
        let mut outstanding = Vec::new();
        for profile in item.profiles() {
            if let Some(reason) = self.ineligibility(profile) {
                if first_sighting {
                    plan.skipped.push(Skip {
                        pair: item.pair(profile),
                        reason,
                    });
                }
                continue;
            }
            eligible += 1;
            match self.answer(&question, profile) {
                Some(finding) if first_sighting => {
                    plan.shallow.extend(self.shallow(finding));
                }
                Some(_) => {}
                None => outstanding.push(profile),
            }
        }
        if first_sighting {
            let index = if outstanding.is_empty() {
                None
            } else {
                self.claim_index(&question)
            };
            if !outstanding.is_empty() && index.is_none() {
                plan.warnings.push(Warning::IndexesExhausted {
                    question: item.question.clone(),
                });
            }
            self.claimed.insert(question, index);
            if let Some(index) = index {
                let slug = QuestionSlug::from(item.question.as_str());
                for profile in outstanding.iter().copied() {
                    let Some(stem) =
                        Stem::allocated(index, slug.as_str(), profile)
                    else {
                        continue;
                    };
                    let stage = self.stage_for(&stem, &item.question, plan);
                    plan.pairs.push(OutstandingPair {
                        pair: item.pair(profile),
                        stem,
                        stage,
                    });
                }
            }
        }
        eligible > 0 && outstanding.is_empty()
    }

    fn stage_for(
        &self,
        stem: &Stem,
        question: &str,
        plan: &mut RoundPlan,
    ) -> Stage {
        let directory = self
            .inputs
            .levels
            .iter()
            .find(|directory| directory.stem() == stem);
        let (stage, trims) =
            derive(stem, question, directory, self.inputs.depth, self.unicode);
        plan.trims.extend(trims);
        stage
    }

    fn shallow(&self, finding: &Finding) -> Option<ShallowFinding> {
        (finding.depth() < self.inputs.depth).then(|| {
            finding.stem().map(|stem| ShallowFinding {
                stem: stem.clone(),
                depth: finding.depth(),
            })
        })?
    }

    fn ineligibility(&self, profile: &str) -> Option<SkipReason> {
        let named = |profiles: &[String]| profiles.iter().any(|p| p == profile);
        if !named(&self.inputs.source_profiles) {
            Some(SkipReason::NotInBrief)
        } else if !named(&self.inputs.available_profiles) {
            Some(SkipReason::NotInstalled)
        } else {
            None
        }
    }

    fn answer(
        &self,
        question: &NormalisedQuestion,
        profile: &str,
    ) -> Option<&'a Finding> {
        self.inputs
            .findings
            .iter()
            .find(|finding| finding.answers(question, profile))
    }

    /// The run's claim first, then the lowest index a retained finding for
    /// the question carries, then the lowest a quarantined file or `.levels/`
    /// directory holds, and only then a fresh one.
    fn claim_index(&mut self, question: &NormalisedQuestion) -> Option<u32> {
        let retained = || {
            self.inputs
                .findings
                .iter()
                .filter(|finding| {
                    finding
                        .answer()
                        .is_some_and(|answer| answer.normalised == *question)
                })
                .filter_map(Finding::index)
                .min()
        };
        let claimed_by_a_file = || {
            self.file_claims
                .iter()
                .filter(|claim| claim.is_for(question))
                .map(IndexClaim::index)
                .min()
        };
        self.inputs
            .claims
            .index_for(question)
            .or_else(retained)
            .or_else(claimed_by_a_file)
            .or_else(|| {
                let index = self.next_unclaimed?;
                self.next_unclaimed = index.checked_add(1);
                Some(index)
            })
    }

    fn unmatched_findings(&self) -> Vec<Warning> {
        let asked: Vec<NormalisedQuestion> = self
            .inputs
            .outline
            .items
            .iter()
            .map(|item| self.normalised(&item.question))
            .collect();
        self.inputs
            .findings
            .iter()
            .filter_map(|finding| {
                let answer = finding.answer()?;
                (!asked.contains(&answer.normalised)).then(|| {
                    Warning::UnmatchedFinding {
                        name: finding.name().to_owned(),
                        question: answer.pair.question.clone(),
                    }
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::OutstandingPair;
    use super::RoundInputs;
    use super::RoundPlan;
    use super::ShallowFinding;
    use super::Skip;
    use super::SkipReason;
    use super::Warning;
    use crate::topic::claims::ClaimedIndexes;
    use crate::topic::claims::IndexClaim;
    use crate::topic::evidence::Digest;
    use crate::topic::evidence::Finding;
    use crate::topic::evidence::LevelNote;
    use crate::topic::evidence::LevelsDirectory;
    use crate::topic::evidence::NoteRejection;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::layout::note_ref::NoteRef;
    use crate::topic::layout::stem::Stem;
    use crate::topic::outline::Outline;
    use crate::topic::outline::Pair;
    use crate::topic::question::FakeUnicode;
    use crate::topic::question::NormalisedQuestion;
    use crate::topic::tree::Depth;
    use crate::topic::tree::Stage;
    use crate::topic::tree::Trim;

    const DIGEST: Digest = Digest::new([9; 32]);

    fn plan(inputs: &RoundInputs) -> RoundPlan {
        RoundPlan::of(inputs, &FakeUnicode)
    }

    fn lineage(text: &str) -> Lineage {
        Lineage::parse(text).unwrap_or_else(Lineage::root)
    }

    fn depth(levels: u32) -> Depth {
        Depth::new(levels).unwrap_or_default()
    }

    fn pair(question: &str, profile: &str) -> Pair {
        Pair {
            question: question.to_owned(),
            profile: profile.to_owned(),
        }
    }

    fn stems(plan: &RoundPlan) -> Vec<String> {
        plan.pairs
            .iter()
            .map(|pair| pair.stem.to_string())
            .collect()
    }

    fn note(at: &str, question: &str, follow_ups: &[&str]) -> LevelNote {
        LevelNote {
            lineage: lineage(at),
            question: question.to_owned(),
            follow_ups: follow_ups.iter().map(|&f| f.to_owned()).collect(),
            digest: DIGEST,
        }
    }

    fn levels(
        stem: &str,
        root_question: Option<&str>,
        notes: Vec<LevelNote>,
    ) -> Vec<LevelsDirectory> {
        Stem::parse(stem)
            .map(|stem| {
                LevelsDirectory::new(
                    stem,
                    root_question.map(str::to_owned),
                    notes,
                    BTreeMap::new(),
                )
            })
            .into_iter()
            .collect()
    }

    fn claims(entries: &[(&str, u32)]) -> ClaimedIndexes {
        let mut claims = ClaimedIndexes::default();
        for (question, index) in entries {
            claims
                .claim(NormalisedQuestion::of(question, &FakeUnicode), *index);
        }
        claims
    }

    fn quarantined(name: &str, question: Option<&str>) -> Vec<IndexClaim> {
        IndexClaim::quarantined(name, question, &FakeUnicode)
            .into_iter()
            .collect()
    }

    fn stage(plan: &RoundPlan) -> Option<&Stage> {
        plan.pairs.first().map(|pair| &pair.stage)
    }

    fn researched_nodes(plan: &RoundPlan) -> Vec<String> {
        match stage(plan) {
            Some(Stage::ResearchNodes(nodes)) => {
                nodes.iter().map(|node| node.lineage.to_string()).collect()
            }
            _ => Vec::new(),
        }
    }

    fn outline(lines: &[&str]) -> Outline {
        Outline::parse(&lines.join("\n"))
    }

    fn retained(name: &str, question: &str, profile: &str) -> Finding {
        Finding::retained(name, pair(question, profile), &FakeUnicode)
    }

    fn inputs(outline: Outline) -> RoundInputs {
        RoundInputs {
            outline,
            findings: Vec::new(),
            quarantined: Vec::new(),
            source_profiles: vec![
                "web".into(),
                "openalex".into(),
                "arxiv".into(),
            ],
            available_profiles: vec![
                "web".into(),
                "openalex".into(),
                "arxiv".into(),
            ],
            levels: Vec::new(),
            depth: Depth::default(),
            claims: ClaimedIndexes::default(),
        }
    }

    fn paths(plan: &RoundPlan) -> Vec<(String, String)> {
        plan.pairs
            .iter()
            .map(|pair| (pair.pair.profile.clone(), pair.stem.finding_path()))
            .collect()
    }

    fn expected(entries: &[(&str, &str)]) -> Vec<(String, String)> {
        entries
            .iter()
            .map(|(profile, path)| ((*profile).to_owned(), (*path).to_owned()))
            .collect()
    }

    #[test]
    fn an_unseparated_profiles_suffix_is_a_warning_and_defaults_to_web() {
        let outline = outline(&["# O", "- [ ] Why? profiles: arxiv"]);
        let plan = plan(&inputs(outline));
        assert_eq!(plan.items[0].question, "Why? profiles: arxiv");
        assert_eq!(
            paths(&plan),
            expected(&[("web", "findings/01-why-profiles-arxiv-web.md")])
        );
        assert_eq!(
            plan.warnings,
            vec![Warning::UnseparatedProfiles { line: 2 }]
        );
    }

    #[test]
    fn a_fresh_multi_profile_item_shares_one_index() {
        let plan =
            plan(&inputs(outline(&["- [ ] How? — profiles: web, openalex"])));
        assert_eq!(
            paths(&plan),
            expected(&[
                ("web", "findings/01-how-web.md"),
                ("openalex", "findings/01-how-openalex.md"),
            ])
        );
        assert!(!plan.items[0].complete);
    }

    #[test]
    fn two_fresh_items_take_consecutive_indices() {
        let plan = plan(&inputs(outline(&["- [ ] A?", "- [ ] B?"])));
        assert_eq!(
            paths(&plan),
            expected(&[
                ("web", "findings/01-a-web.md"),
                ("web", "findings/02-b-web.md")
            ])
        );
    }

    #[test]
    fn a_gap_fill_reuses_the_index_of_the_retained_finding() {
        let mut inputs = inputs(outline(&[
            "- [ ] A?",
            "- [ ] B? — profiles: web, openalex",
        ]));
        inputs.findings = vec![
            retained("01-a-web.md", "A?", "web"),
            retained("03-b-web.md", "B?", "web"),
        ];
        let plan = plan(&inputs);
        assert_eq!(
            paths(&plan),
            expected(&[("openalex", "findings/03-b-openalex.md")])
        );
        assert!(plan.items[0].complete);
        assert!(!plan.items[1].complete);
    }

    #[test]
    fn a_quarantined_pair_reuses_its_markers_index() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] C?"]));
        inputs.findings = vec![retained("01-a-web.md", "A?", "web")];
        inputs.quarantined = quarantined(".04-c-web.md.invalid", Some("C?"));
        let plan = plan(&inputs);
        assert_eq!(paths(&plan), expected(&[("web", "findings/04-c-web.md")]));
    }

    #[test]
    fn a_new_focus_area_never_reuses_an_index_a_marker_holds() {
        let mut inputs = inputs(outline(&["- [ ] New?"]));
        inputs.quarantined = quarantined(".02-old-web.md.invalid", None);
        let plan = plan(&inputs);
        assert_eq!(
            paths(&plan),
            expected(&[("web", "findings/03-new-web.md")])
        );
    }

    #[test]
    fn an_invalid_finding_holds_its_index_but_leaves_its_pair_outstanding() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] B?"]));
        inputs.findings = vec![Finding::invalid("01-a-web.md")];
        let plan = plan(&inputs);
        assert_eq!(
            paths(&plan),
            expected(&[
                ("web", "findings/02-a-web.md"),
                ("web", "findings/03-b-web.md")
            ])
        );
    }

    #[test]
    fn a_hand_named_invalid_finding_still_holds_its_index() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.findings = vec![Finding::invalid("05-Hand Named.md")];
        assert_eq!(stems(&plan(&inputs)), ["06-a-web"]);
    }

    #[test]
    fn a_profile_outside_the_brief_is_skipped() {
        let mut inputs = inputs(outline(&["- [ ] A? — profiles: web, arxiv"]));
        inputs.source_profiles = vec!["web".into()];
        let plan = plan(&inputs);
        assert_eq!(paths(&plan), expected(&[("web", "findings/01-a-web.md")]));
        assert_eq!(
            plan.skipped,
            vec![Skip {
                pair: pair("A?", "arxiv"),
                reason: SkipReason::NotInBrief,
            }]
        );
    }

    #[test]
    fn a_profile_with_no_skill_is_skipped() {
        let mut inputs = inputs(outline(&["- [ ] A? — profiles: crossref"]));
        inputs.source_profiles.push("crossref".into());
        let plan = plan(&inputs);
        assert!(plan.pairs.is_empty());
        assert_eq!(
            plan.skipped,
            vec![Skip {
                pair: pair("A?", "crossref"),
                reason: SkipReason::NotInstalled,
            }]
        );
    }

    #[test]
    fn an_item_whose_pairs_are_all_skipped_is_never_complete() {
        let mut inputs = inputs(outline(&["- [x] A? — profiles: arxiv"]));
        inputs.source_profiles = vec!["web".into()];
        let plan = plan(&inputs);
        assert!(!plan.items[0].complete);
    }

    #[test]
    fn a_skipped_pair_does_not_hold_back_an_otherwise_complete_item() {
        let mut inputs = inputs(outline(&["- [ ] A? — profiles: web, arxiv"]));
        inputs.source_profiles = vec!["web".into()];
        inputs.findings = vec![retained("01-a-web.md", "A?", "web")];
        let plan = plan(&inputs);
        assert!(plan.items[0].complete);
    }

    #[test]
    fn a_legacy_flat_finding_completes_its_web_pair() {
        let mut inputs =
            inputs(outline(&["- [x] What is the first focus area?"]));
        inputs.findings = vec![retained(
            "01-first-focus.md",
            "What is the first focus area?",
            "web",
        )];
        let plan = plan(&inputs);
        assert!(plan.pairs.is_empty());
        assert!(plan.items[0].complete);
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn a_non_latin_question_is_allocated_a_focus_area_slug() {
        let plan = plan(&inputs(outline(&["- [ ] 注意力机制？"])));
        assert_eq!(
            paths(&plan),
            expected(&[("web", "findings/01-focus-area-web.md")])
        );
    }

    #[test]
    fn questions_match_after_collapsing_whitespace() {
        let mut inputs =
            inputs(outline(&["- [ ] How   do heads\tspecialise?"]));
        inputs.findings =
            vec![retained("01-x.md", " How do heads specialise? ", "web")];
        let plan = plan(&inputs);
        assert!(plan.items[0].complete);
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn a_finding_answering_no_outline_question_is_a_warning() {
        let mut inputs = inputs(outline(&["- [ ] How do heads specialise?"]));
        inputs.findings =
            vec![retained("01-x.md", "How do the heads specialise?", "web")];
        let plan = plan(&inputs);
        assert!(!plan.items[0].complete);
        assert_eq!(
            paths(&plan),
            expected(&[("web", "findings/02-how-do-heads-specialise-web.md")])
        );
        assert_eq!(
            plan.warnings,
            vec![Warning::UnmatchedFinding {
                name: "01-x.md".into(),
                question: "How do the heads specialise?".into(),
            }]
        );
    }

    #[test]
    fn the_lowest_retained_index_of_a_focus_area_wins() {
        let mut inputs =
            inputs(outline(&["- [ ] A? — profiles: web, openalex, arxiv"]));
        inputs.findings = vec![
            retained("05-a-web.md", "A?", "web"),
            retained("02-a-openalex.md", "A?", "openalex"),
        ];
        let plan = plan(&inputs);
        assert_eq!(
            paths(&plan),
            expected(&[("arxiv", "findings/02-a-arxiv.md")])
        );
    }

    #[test]
    fn a_ticked_item_that_is_newly_incomplete_reports_incomplete() {
        let plan =
            plan(&inputs(outline(&["- [x] A? — profiles: web, openalex"])));
        assert!(!plan.items[0].complete);
        assert_eq!(plan.pairs.len(), 2);
    }

    #[test]
    fn a_question_repeated_across_rounds_is_allocated_once() {
        let plan = plan(&inputs(outline(&["- [ ] A?", "- [ ] A?"])));
        assert_eq!(plan.items.len(), 2);
        assert_eq!(
            plan.pairs,
            Stem::parse("01-a-web")
                .map(|stem| OutstandingPair {
                    pair: pair("A?", "web"),
                    stem,
                    stage: Stage::SinglePass,
                })
                .into_iter()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_pair_without_levels_at_depth_one_is_researched_in_a_single_pass() {
        let plan = plan(&inputs(outline(&["- [ ] A?"])));
        assert_eq!(stems(&plan), ["01-a-web"]);
        assert_eq!(stage(&plan), Some(&Stage::SinglePass));
        assert!(plan.trims.is_empty());
        assert!(plan.shallow.is_empty());
    }

    #[test]
    fn a_pair_at_depth_two_with_no_levels_researches_its_root_node() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(2);
        let plan = plan(&inputs);
        assert_eq!(stems(&plan), ["01-a-web"]);
        assert_eq!(researched_nodes(&plan), ["1"]);
    }

    #[test]
    fn a_pair_whose_tree_is_complete_awaits_only_composition() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(2);
        inputs.levels = levels(
            "01-a-web",
            Some("A?"),
            vec![note("1", "A?", &["B?"]), note("2-1", "B?", &["C?"])],
        );
        let plan = plan(&inputs);
        assert_eq!(
            stage(&plan),
            Some(&Stage::Compose(vec![lineage("1"), lineage("2-1")]))
        );
    }

    #[test]
    fn a_pair_at_depth_one_with_a_root_note_composes_from_it() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.levels =
            levels("01-a-web", Some("A?"), vec![note("1", "A?", &["B?"])]);
        let plan = plan(&inputs);
        assert_eq!(stage(&plan), Some(&Stage::Compose(vec![lineage("1")])));
    }

    #[test]
    fn a_levels_directory_without_a_root_at_depth_one_researches_its_root() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.levels = levels("01-a-web", Some("A?"), Vec::new());
        let plan = plan(&inputs);
        assert_eq!(stems(&plan), ["01-a-web"]);
        assert_eq!(researched_nodes(&plan), ["1"]);
    }

    #[test]
    fn a_levels_directory_holds_its_index_through_its_root_note() {
        let mut inputs = inputs(outline(&["- [ ] Z?", "- [ ] A?"]));
        inputs.levels =
            levels("04-a-web", Some("A?"), vec![note("1", "A?", &[])]);
        let plan = plan(&inputs);
        assert_eq!(stems(&plan), ["05-z-web", "04-a-web"]);
    }

    #[test]
    fn a_levels_directory_holds_its_index_through_a_quarantined_root_note() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.levels = levels("04-a-web", Some("A?"), Vec::new());
        assert_eq!(stems(&plan(&inputs)), ["04-a-web"]);
    }

    #[test]
    fn a_levels_directory_holds_its_index_through_an_unaccepted_root_note() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.levels = Stem::parse("04-a-web")
            .map(|stem| {
                LevelsDirectory::new(
                    stem,
                    Some("A?".into()),
                    Vec::new(),
                    BTreeMap::from([(
                        Lineage::root(),
                        NoteRejection::WrongKind,
                    )]),
                )
            })
            .into_iter()
            .collect();
        let plan = plan(&inputs);
        assert_eq!(stems(&plan), ["04-a-web"]);
        let Some(Stage::ResearchNodes(nodes)) = stage(&plan) else {
            return assert_eq!(stage(&plan), None, "a research-nodes stage");
        };
        assert_eq!(nodes[0].rejected, Some(NoteRejection::WrongKind));
    }

    #[test]
    fn every_profile_of_a_focus_area_resumes_at_the_index_its_levels_directory_holds(
    ) {
        let mut inputs =
            inputs(outline(&["- [ ] A? — profiles: web, openalex"]));
        inputs.levels = levels("04-a-web", Some("A?"), Vec::new());
        assert_eq!(stems(&plan(&inputs)), ["04-a-web", "04-a-openalex"]);
    }

    #[test]
    fn a_levels_directory_without_a_root_note_holds_its_index_only_against_new_allocations(
    ) {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] B?"]));
        inputs.levels = levels("04-a-web", None, Vec::new());
        assert_eq!(stems(&plan(&inputs)), ["05-a-web", "06-b-web"]);
    }

    #[test]
    fn the_lowest_index_a_marker_or_levels_directory_holds_wins() {
        let mut inputs =
            inputs(outline(&["- [ ] A? — profiles: web, openalex"]));
        inputs.quarantined = quarantined(".06-a-web.md.invalid", Some("A?"));
        inputs.levels = levels("04-a-openalex", Some("A?"), Vec::new());
        assert_eq!(stems(&plan(&inputs)), ["04-a-web", "04-a-openalex"]);
        inputs.quarantined = quarantined(".02-a-web.md.invalid", Some("A?"));
        assert_eq!(stems(&plan(&inputs)), ["02-a-web", "02-a-openalex"]);
    }

    #[test]
    fn a_retained_findings_index_wins_over_one_a_file_holds() {
        let mut inputs =
            inputs(outline(&["- [ ] A? — profiles: web, openalex"]));
        inputs.findings = vec![retained("05-a-web.md", "A?", "web")];
        inputs.quarantined =
            quarantined(".02-a-openalex.md.invalid", Some("A?"));
        assert_eq!(stems(&plan(&inputs)), ["05-a-openalex"]);
    }

    #[test]
    fn an_index_at_the_ceiling_never_overflows_allocation() {
        let mut inputs = inputs(outline(&["- [ ] M?", "- [ ] N?"]));
        inputs.levels = levels("4294967295-x-web", None, Vec::new());
        let plan_at_ceiling = plan(&inputs);
        assert!(plan_at_ceiling.pairs.is_empty());
        assert_eq!(
            plan_at_ceiling.warnings,
            vec![
                Warning::IndexesExhausted {
                    question: "M?".into()
                },
                Warning::IndexesExhausted {
                    question: "N?".into()
                },
            ]
        );

        inputs.levels = levels("4294967294-x-web", None, Vec::new());
        let plan = plan(&inputs);
        assert_eq!(stems(&plan), ["4294967295-m-web"]);
        assert_eq!(
            plan.warnings,
            vec![Warning::IndexesExhausted {
                question: "N?".into()
            }]
        );
    }

    #[test]
    fn a_new_focus_area_never_takes_an_index_a_levels_directory_holds() {
        let mut inputs = inputs(outline(&["- [ ] New?"]));
        inputs.levels = levels("02-old-web", Some("Old?"), Vec::new());
        assert_eq!(stems(&plan(&inputs)), ["03-new-web"]);
    }

    #[test]
    fn a_pair_with_a_retained_finding_is_never_deepened() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(3);
        inputs.findings = vec![retained("01-a-web.md", "A?", "web")];
        inputs.levels =
            levels("01-a-web", Some("A?"), vec![note("1", "A?", &["B?"])]);
        let plan = plan(&inputs);
        assert!(plan.pairs.is_empty());
        assert!(plan.items[0].complete);
    }

    #[test]
    fn a_retained_finding_below_the_requested_depth_is_reported_shallow() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(3);
        inputs.findings =
            vec![retained("01-a-web.md", "A?", "web").with_depth(depth(2))];
        let plan = plan(&inputs);
        assert_eq!(
            plan.shallow,
            Stem::parse("01-a-web")
                .map(|stem| ShallowFinding {
                    stem,
                    depth: depth(2)
                })
                .into_iter()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_legacy_finding_is_reported_shallow_against_depth_two() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(2);
        inputs.findings = vec![retained("01-a-web.md", "A?", "web")];
        let plan_at_two = plan(&inputs);
        assert_eq!(plan_at_two.shallow.len(), 1);
        assert_eq!(plan_at_two.shallow[0].depth, Depth::default());
        inputs.depth = Depth::default();
        assert!(plan(&inputs).shallow.is_empty());
    }

    #[test]
    fn a_retained_finding_at_the_requested_depth_is_not_reported_shallow() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] A?"]));
        inputs.depth = depth(3);
        inputs.findings =
            vec![retained("01-a-web.md", "A?", "web").with_depth(depth(3))];
        assert!(plan(&inputs).shallow.is_empty());
        inputs.findings =
            vec![retained("01-a-web.md", "A?", "web").with_depth(depth(1))];
        assert_eq!(plan(&inputs).shallow.len(), 1);
    }

    #[test]
    fn a_failed_pairs_stem_survives_a_later_pair_writing_within_a_run() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] B?"]));
        inputs.findings = vec![retained("02-b-web.md", "B?", "web")];
        assert_eq!(stems(&plan(&inputs)), ["03-a-web"]);
        inputs.claims = claims(&[("A?", 1), ("B?", 2)]);
        assert_eq!(stems(&plan(&inputs)), ["01-a-web"]);
    }

    #[test]
    fn a_pair_keeps_its_stem_when_its_root_note_records_another_question() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(2);
        inputs.levels =
            levels("01-a-web", Some("Other?"), vec![note("1", "Other?", &[])]);
        assert_eq!(stems(&plan(&inputs)), ["02-a-web"]);
        inputs.claims = claims(&[("A?", 1)]);
        let plan = plan(&inputs);
        assert_eq!(stems(&plan), ["01-a-web"]);
        let Some(Stage::ResearchNodes(nodes)) = stage(&plan) else {
            return assert_eq!(stage(&plan), None, "a research-nodes stage");
        };
        assert_eq!(
            nodes[0].rejected,
            Some(NoteRejection::QuestionDisagreesWithCandidate)
        );
    }

    #[test]
    fn a_new_focus_area_never_takes_a_claimed_index() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] New?"]));
        inputs.claims = claims(&[("A?", 5)]);
        assert_eq!(stems(&plan(&inputs)), ["05-a-web", "06-new-web"]);
    }

    #[test]
    fn a_plan_claims_the_index_each_planned_focus_area_holds() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] B?", "- [ ] C?"]));
        inputs.findings = vec![retained("02-b-web.md", "B?", "web")];
        let plan = plan(&inputs);
        assert_eq!(plan.claims, claims(&[("A?", 3), ("C?", 4)]));
    }

    #[test]
    fn trims_name_the_note_they_trim() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(2);
        inputs.levels = levels(
            "01-a-web",
            Some("A?"),
            vec![note("1", "A?", &["B?", "C?", "D?", "E?", "F?"])],
        );
        let plan = plan(&inputs);
        assert_eq!(
            plan.trims,
            Stem::parse("01-a-web")
                .map(|stem| Trim {
                    at: NoteRef {
                        stem,
                        lineage: Lineage::root(),
                    },
                    recorded: 5,
                    cap: 4,
                })
                .into_iter()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn known_questions_render_candidate_text_not_a_notes_own_question() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(3);
        inputs.levels =
            levels("01-a-web", Some("A?"), vec![note("1", "  A?\n", &["B?"])]);
        let plan = plan(&inputs);
        let Some(Stage::ResearchNodes(nodes)) = stage(&plan) else {
            return assert_eq!(stage(&plan), None, "a research-nodes stage");
        };
        assert_eq!(nodes[0].lineage, lineage("2-1"));
        assert_eq!(nodes[0].known_questions, ["A?", "B?"]);
    }

    #[test]
    fn a_finding_answers_an_outline_item_equal_under_compatibility_folding() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.findings = vec![retained("01-a-web.md", "A\u{FF1F}", "web")];
        let plan = plan(&inputs);
        assert!(plan.items[0].complete);
        assert!(plan.pairs.is_empty());
        assert!(plan.warnings.is_empty());
    }
}
