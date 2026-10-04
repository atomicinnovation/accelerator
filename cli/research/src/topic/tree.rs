//! A pair's research tree: which of its nodes are still missing a level note,
//! and, once none is, which notes its finding is composed from.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::num::NonZeroU32;

use crate::topic::evidence::LevelNote;
use crate::topic::evidence::LevelsDirectory;
use crate::topic::evidence::NoteRejection;
use crate::topic::layout::lineage::Lineage;
use crate::topic::layout::note_ref::NoteRef;
use crate::topic::layout::stem::Stem;
use crate::topic::question::NormalisedQuestion;
use crate::topic::question::UnicodeText;

pub const STARTING_FOLLOW_UP_CAP: u32 = 4;

/// How many levels of a pair's tree are researched; one is the root alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Depth(u32);

impl Depth {
    #[must_use]
    pub const fn new(levels: u32) -> Option<Self> {
        if levels == 0 {
            None
        } else {
            Some(Self(levels))
        }
    }

    #[must_use]
    pub const fn levels(self) -> u32 {
        self.0
    }
}

impl Default for Depth {
    fn default() -> Self {
        Self(1)
    }
}

/// The most follow-ups a note at `level` may record: halving per level,
/// rounding up, from the starting cap at the root.
#[must_use]
pub fn follow_up_cap(level: u32) -> u32 {
    let mut cap = STARTING_FOLLOW_UP_CAP;
    for _ in 1..level {
        if cap == 1 {
            break;
        }
        cap = cap.div_ceil(2);
    }
    cap
}

/// What a pair needs next: one researcher writing its finding in a single
/// pass, researchers for the nodes its tree is missing, or the composer over
/// its complete tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    SinglePass,
    ResearchNodes(Vec<MissingNode>),
    Compose(Vec<Lineage>),
}

/// A node still missing its level note, with what its researcher is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingNode {
    pub lineage: Lineage,
    pub question: String,
    pub known_questions: Vec<String>,
    pub rejected: Option<NoteRejection>,
}

impl MissingNode {
    /// The most follow-ups this node's note may record.
    #[must_use]
    pub fn cap(&self) -> u32 {
        follow_up_cap(self.lineage.level())
    }
}

/// A note that recorded more follow-ups than its cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trim {
    pub at: NoteRef,
    pub recorded: u32,
    pub cap: u32,
}

impl Trim {
    #[must_use]
    pub const fn trimmed(&self) -> u32 {
        self.recorded.saturating_sub(self.cap)
    }
}

/// The stage of the pair named `stem`, whose tree is rooted at
/// `pair_question` and derived level by level down to `depth` from the notes
/// in `directory`, with the notes trimmed on the way.
///
/// At depth 1 a pair with no `.levels/` directory is researched in a single
/// pass. A level is derived only once every node above it has its note, so a
/// node's candidates never depend on which siblings finished first.
#[must_use]
pub fn derive(
    stem: &Stem,
    pair_question: &str,
    directory: Option<&LevelsDirectory>,
    depth: Depth,
    unicode: &dyn UnicodeText,
) -> (Stage, Vec<Trim>) {
    if directory.is_none() && depth == Depth::default() {
        return (Stage::SinglePass, Vec::new());
    }
    TreeWalk::new(stem, pair_question, directory, unicode).walk(depth)
}

struct TreeWalk<'a> {
    unicode: &'a dyn UnicodeText,
    stem: &'a Stem,
    pair_question: &'a str,
    notes: BTreeMap<&'a Lineage, &'a LevelNote>,
    rejected: Option<&'a BTreeMap<Lineage, NoteRejection>>,
    questions: BTreeMap<Lineage, String>,
    recorded: Vec<String>,
    trims: Vec<Trim>,
}

impl<'a> TreeWalk<'a> {
    fn new(
        stem: &'a Stem,
        pair_question: &'a str,
        directory: Option<&'a LevelsDirectory>,
        unicode: &'a dyn UnicodeText,
    ) -> Self {
        let notes = directory
            .map(|directory| {
                directory
                    .notes()
                    .iter()
                    .map(|note| (&note.lineage, note))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            unicode,
            stem,
            pair_question,
            notes,
            rejected: directory.map(LevelsDirectory::rejected),
            questions: BTreeMap::new(),
            recorded: Vec::new(),
            trims: Vec::new(),
        }
    }

    fn walk(mut self, depth: Depth) -> (Stage, Vec<Trim>) {
        let mut known = BTreeSet::from([self.normalised(self.pair_question)]);
        self.questions
            .insert(Lineage::root(), self.pair_question.to_owned());
        let mut candidates = vec![Lineage::root()];
        let mut reached = Vec::new();
        for level in 1..=depth.levels() {
            if candidates.is_empty() {
                break;
            }
            for lineage in &candidates {
                known.insert(self.normalised(self.question_at(lineage)));
            }
            let (notes, missing) = self.answer(&candidates);
            if !missing.is_empty() {
                return (Stage::ResearchNodes(missing), self.trims);
            }
            reached.append(&mut candidates);
            if level < depth.levels() {
                candidates = self.follow_ups(&notes, level, &mut known);
            }
        }
        (Stage::Compose(reached), self.trims)
    }

    fn question_at(&self, lineage: &Lineage) -> &str {
        self.questions.get(lineage).map_or("", String::as_str)
    }

    fn answer(
        &self,
        candidates: &[Lineage],
    ) -> (Vec<&'a LevelNote>, Vec<MissingNode>) {
        let mut notes = Vec::new();
        let mut missing = Vec::new();
        for lineage in candidates {
            let wanted = self.normalised(self.question_at(lineage));
            match self.notes.get(lineage) {
                Some(note) if self.normalised(&note.question) == wanted => {
                    notes.push(*note);
                }
                Some(_) => missing.push(self.missing(
                    lineage,
                    Some(NoteRejection::QuestionDisagreesWithCandidate),
                )),
                None => missing.push(
                    self.missing(
                        lineage,
                        self.rejected
                            .and_then(|rejected| rejected.get(lineage))
                            .cloned(),
                    ),
                ),
            }
        }
        (notes, missing)
    }

    fn follow_ups(
        &mut self,
        notes: &[&LevelNote],
        level: u32,
        known: &mut BTreeSet<NormalisedQuestion>,
    ) -> Vec<Lineage> {
        let cap = follow_up_cap(level);
        let mut candidates = Vec::new();
        for note in notes {
            let recorded =
                u32::try_from(note.follow_ups.len()).unwrap_or(u32::MAX);
            if recorded > cap {
                self.trims.push(Trim {
                    at: NoteRef {
                        stem: self.stem.clone(),
                        lineage: note.lineage.clone(),
                    },
                    recorded,
                    cap,
                });
            }
            let capped = note.follow_ups.iter().take(cap as usize);
            for (position, follow_up) in (1..).zip(capped) {
                self.recorded.push(follow_up.clone());
                let Some(position) = NonZeroU32::new(position) else {
                    continue;
                };
                if known.insert(self.normalised(follow_up)) {
                    let lineage = note.lineage.child(position);
                    self.questions.insert(lineage.clone(), follow_up.clone());
                    candidates.push(lineage);
                }
            }
        }
        candidates
    }

    fn missing(
        &self,
        lineage: &Lineage,
        rejected: Option<NoteRejection>,
    ) -> MissingNode {
        MissingNode {
            lineage: lineage.clone(),
            question: self.question_at(lineage).to_owned(),
            known_questions: self.known_questions(lineage),
            rejected,
        }
    }

    fn known_questions(&self, lineage: &Lineage) -> Vec<String> {
        let ancestors =
            std::iter::successors(lineage.parent(), Lineage::parent)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .filter_map(|ancestor| self.questions.get(&ancestor).cloned());
        let mut seen = BTreeSet::new();
        std::iter::once(self.pair_question.to_owned())
            .chain(ancestors)
            .chain(self.recorded.iter().cloned())
            .filter(|question| seen.insert(self.normalised(question)))
            .collect()
    }

    fn normalised(&self, question: &str) -> NormalisedQuestion {
        NormalisedQuestion::of(question, self.unicode)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::derive;
    use super::follow_up_cap;
    use super::Depth;
    use super::MissingNode;
    use super::Stage;
    use super::Trim;
    use crate::topic::evidence::Digest;
    use crate::topic::evidence::LevelNote;
    use crate::topic::evidence::LevelsDirectory;
    use crate::topic::evidence::NoteRejection;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::layout::slug::QuestionSlug;
    use crate::topic::layout::stem::Stem;
    use crate::topic::question::FakeUnicode;

    const PAIR: &str = "P?";

    fn lineage(text: &str) -> Lineage {
        Lineage::parse(text).unwrap_or_else(Lineage::root)
    }

    fn depth(levels: u32) -> Depth {
        Depth::new(levels).unwrap_or_default()
    }

    fn stem() -> Option<Stem> {
        Stem::allocated(3, QuestionSlug::from(PAIR).as_str(), "web")
    }

    fn note(at: &str, question: &str, follow_ups: &[&str]) -> LevelNote {
        LevelNote {
            lineage: lineage(at),
            question: question.to_owned(),
            follow_ups: follow_ups.iter().map(|&f| f.to_owned()).collect(),
            digest: Digest::new([0; 32]),
        }
    }

    fn directory(
        notes: Vec<LevelNote>,
        rejected: &[(&str, NoteRejection)],
    ) -> Option<LevelsDirectory> {
        Some(LevelsDirectory::new(
            stem()?,
            Some(PAIR.to_owned()),
            notes,
            rejected
                .iter()
                .map(|(at, rejection)| (lineage(at), rejection.clone()))
                .collect::<BTreeMap<_, _>>(),
        ))
    }

    fn derived_from(
        directory: Option<&LevelsDirectory>,
        levels: u32,
    ) -> (Stage, Vec<Trim>) {
        stem().map_or((Stage::SinglePass, Vec::new()), |stem| {
            derive(&stem, PAIR, directory, depth(levels), &FakeUnicode)
        })
    }

    fn derived(notes: Vec<LevelNote>, levels: u32) -> (Stage, Vec<Trim>) {
        derived_from(directory(notes, &[]).as_ref(), levels)
    }

    fn nodes(stage: Stage) -> Vec<MissingNode> {
        match stage {
            Stage::ResearchNodes(nodes) => nodes,
            Stage::SinglePass | Stage::Compose(_) => Vec::new(),
        }
    }

    fn missing(derivation: &(Stage, Vec<Trim>)) -> Vec<String> {
        nodes(derivation.0.clone())
            .iter()
            .map(|node| node.lineage.to_string())
            .collect()
    }

    fn compose(lineages: &[&str]) -> Stage {
        Stage::Compose(lineages.iter().map(|at| lineage(at)).collect())
    }

    fn known(derivation: &(Stage, Vec<Trim>), at: &str) -> Vec<String> {
        nodes(derivation.0.clone())
            .into_iter()
            .find(|node| node.lineage == lineage(at))
            .map(|node| node.known_questions)
            .unwrap_or_default()
    }

    #[test]
    fn a_depth_of_zero_is_not_a_depth() {
        assert_eq!(Depth::new(0), None);
        assert_eq!(Depth::new(3).map(Depth::levels), Some(3));
        assert_eq!(Depth::default().levels(), 1);
    }

    #[test]
    fn a_pair_without_levels_at_depth_one_is_researched_in_a_single_pass() {
        assert_eq!(derived_from(None, 1), (Stage::SinglePass, Vec::new()));
    }

    #[test]
    fn a_pair_with_no_notes_is_missing_its_root() {
        let nodes = nodes(derived_from(None, 3).0);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].lineage, Lineage::root());
        assert_eq!(nodes[0].question, PAIR);
        assert_eq!(nodes[0].cap(), 4);
        assert_eq!(nodes[0].known_questions, vec![PAIR.to_owned()]);
        assert_eq!(nodes[0].rejected, None);
    }

    #[test]
    fn caps_halve_per_level_rounding_up() {
        let caps: Vec<u32> = (1..=5).map(follow_up_cap).collect();
        assert_eq!(caps, vec![4, 2, 1, 1, 1]);
        assert_eq!(follow_up_cap(u32::MAX), 1);
    }

    #[test]
    fn an_over_cap_note_is_trimmed_to_its_first_cap_entries() {
        let roots = ["B1", "B2", "B3", "B4", "B5", "B6"];
        let mut notes = vec![note("1", PAIR, &roots)];
        for k in 1..=4 {
            let own: Vec<String> =
                (1..=3).map(|j| format!("X{k}{j}")).collect();
            let own: Vec<&str> = own.iter().map(String::as_str).collect();
            notes.push(note(&format!("2-{k}"), roots[k - 1], &own));
        }
        let derivation = derived(notes, 3);
        assert_eq!(
            missing(&derivation),
            [
                "3-1-1", "3-1-2", "3-2-1", "3-2-2", "3-3-1", "3-3-2", "3-4-1",
                "3-4-2"
            ]
        );
        let trims: Vec<(String, u32, u32, u32)> = derivation
            .1
            .iter()
            .map(|t| (t.at.to_string(), t.recorded, t.cap, t.trimmed()))
            .collect();
        assert_eq!(
            trims,
            vec![
                ("03-p-web:1".to_owned(), 6, 4, 2),
                ("03-p-web:2-1".to_owned(), 3, 2, 1),
                ("03-p-web:2-2".to_owned(), 3, 2, 1),
                ("03-p-web:2-3".to_owned(), 3, 2, 1),
                ("03-p-web:2-4".to_owned(), 3, 2, 1),
            ]
        );
    }

    #[test]
    fn a_follow_up_matching_the_pair_question_is_skipped_and_siblings_keep_positions(
    ) {
        let derivation = derived(vec![note("1", PAIR, &[PAIR, "B", "C"])], 2);
        assert_eq!(missing(&derivation), ["2-2", "2-3"]);
    }

    #[test]
    fn a_repeated_follow_up_keeps_its_lowest_lineage() {
        let derivation = derived(vec![note("1", PAIR, &["B", "B"])], 2);
        assert_eq!(missing(&derivation), ["2-1"]);
    }

    #[test]
    fn a_follow_up_matching_an_earlier_candidate_at_its_level_is_skipped() {
        let derivation = derived(
            vec![
                note("1", PAIR, &["B", "C"]),
                note("2-1", "B", &["X"]),
                note("2-2", "C", &["X", "Y"]),
            ],
            3,
        );
        assert_eq!(missing(&derivation), ["3-1-1", "3-2-2"]);
    }

    #[test]
    fn a_follow_up_matching_a_shallower_node_is_skipped() {
        let derivation = derived(
            vec![
                note("1", PAIR, &["B", "C"]),
                note("2-1", "B", &["C"]),
                note("2-2", "C", &[]),
            ],
            3,
        );
        assert_eq!(derivation.0, compose(&["1", "2-1", "2-2"]));
    }

    #[test]
    fn questions_collapse_under_normalised_whitespace() {
        let derivation = derived(vec![note("1", PAIR, &["B", " B "])], 2);
        assert_eq!(missing(&derivation), ["2-1"]);
    }

    #[test]
    fn a_level_waits_for_every_note_above_it() {
        let derivation = derived(
            vec![
                note("1", PAIR, &["B", "C", "D", "E"]),
                note("2-1", "B", &["X"]),
                note("2-2", "C", &["Y"]),
                note("2-3", "D", &["Z"]),
            ],
            3,
        );
        assert_eq!(missing(&derivation), ["2-4"]);
    }

    #[test]
    fn an_empty_follow_ups_prunes_its_node() {
        let derivation = derived(vec![note("1", PAIR, &[])], 3);
        assert_eq!(derivation.0, compose(&["1"]));
    }

    #[test]
    fn limit_level_follow_ups_derive_nothing() {
        let derivation = derived(
            vec![note("1", PAIR, &["B"]), note("2-1", "B", &["X", "Y", "Z"])],
            2,
        );
        assert_eq!(derivation.0, compose(&["1", "2-1"]));
        assert!(derivation.1.is_empty());
    }

    #[test]
    fn a_smaller_depth_composes_from_notes_within_it() {
        let derivation = derived(
            vec![
                note("1", PAIR, &["B"]),
                note("2-1", "B", &["X"]),
                note("3-1-1", "X", &[]),
            ],
            1,
        );
        assert_eq!(derivation.0, compose(&["1"]));
    }

    #[test]
    fn a_larger_depth_extends_from_limit_level_follow_ups() {
        let derivation = derived(
            vec![
                note("1", PAIR, &["B", "C"]),
                note("2-1", "B", &["X"]),
                note("2-2", "C", &["Y", "Z"]),
            ],
            3,
        );
        assert_eq!(missing(&derivation), ["3-1-1", "3-2-1", "3-2-2"]);
    }

    #[test]
    fn known_questions_list_the_pair_ancestors_and_recorded_follow_ups() {
        let derivation =
            derived(vec![note("1", PAIR, &["B"]), note("2-1", "B", &["X"])], 3);
        assert_eq!(known(&derivation, "3-1-1"), [PAIR, "B", "X"]);
    }

    #[test]
    fn known_questions_ignore_follow_ups_of_notes_at_the_nodes_own_level() {
        let derivation = derived(
            vec![note("1", PAIR, &["B", "C"]), note("2-1", "B", &["X"])],
            3,
        );
        assert_eq!(missing(&derivation), ["2-2"]);
        assert_eq!(known(&derivation, "2-2"), [PAIR, "B", "C"]);
    }

    #[test]
    fn a_skipped_follow_up_is_not_backfilled_from_past_the_cap() {
        let derivation =
            derived(vec![note("1", PAIR, &[PAIR, "B", "C", "D", "E"])], 2);
        assert_eq!(missing(&derivation), ["2-2", "2-3", "2-4"]);
    }

    #[test]
    fn a_missing_node_carries_the_rejection_of_its_note() {
        let rejection = NoteRejection::LevelDisagreesWithLineage;
        let directory = directory(
            vec![note("1", PAIR, &["B"])],
            &[("2-1", rejection.clone())],
        );
        let nodes = nodes(derived_from(directory.as_ref(), 2).0);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].lineage, lineage("2-1"));
        assert_eq!(nodes[0].rejected, Some(rejection));
    }

    #[test]
    fn a_note_answering_a_different_question_is_missing() {
        let nodes = nodes(
            derived(vec![note("1", PAIR, &["B"]), note("2-1", "X", &[])], 2).0,
        );
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].lineage, lineage("2-1"));
        assert_eq!(nodes[0].question, "B");
        assert_eq!(
            nodes[0].rejected,
            Some(NoteRejection::QuestionDisagreesWithCandidate)
        );
    }

    #[test]
    fn a_note_at_a_lineage_no_candidate_occupies_is_neither_composed_nor_counted(
    ) {
        let derivation = derived(
            vec![
                note("1", PAIR, &[PAIR, "B"]),
                note("2-1", "A", &[]),
                note("2-2", "B", &[]),
                note("2-5", "C", &[]),
            ],
            2,
        );
        assert_eq!(derivation.0, compose(&["1", "2-2"]));
    }
}
