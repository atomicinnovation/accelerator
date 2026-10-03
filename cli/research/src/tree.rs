//! A pair's research tree: which of its nodes are still missing a level note,
//! and, once none is, which notes its finding is composed from.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use crate::lineage::Lineage;
use crate::question::plain_question_breach;
use crate::question::NormalisedQuestion;
use crate::question::PlainQuestionRule;
use crate::question::UnicodeText;
use crate::question::LONGEST_PLAIN_QUESTION;
use crate::stem::Stem;

pub const STARTING_FOLLOW_UP_CAP: u32 = 4;

const LEVEL_NOTE_KIND: &str = "level-note";

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

/// The SHA-256 of a level note's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Which pair's tree a level note belongs to, and where in it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteRef {
    pub stem: Stem,
    pub lineage: Lineage,
}

impl fmt::Display for NoteRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.stem, self.lineage)
    }
}

/// A schema-valid level note that counts towards its tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelNote {
    pub lineage: Lineage,
    pub question: String,
    pub follow_ups: Vec<String>,
    pub digest: Digest,
}

/// Why the note at a lineage does not count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteRejection {
    FailsValidation {
        code: &'static str,
        field: Option<&'static str>,
    },
    WrongKind,
    LevelDisagreesWithLineage,
    MissingQuestion,
    MalformedFollowUps,
    NotAPlainQuestion {
        follow_up: NonZeroUsize,
        rule: PlainQuestionRule,
    },
    QuestionDisagreesWithCandidate,
}

impl fmt::Display for NoteRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FailsValidation { code, field: None } => {
                write!(f, "fails validation: {code}")
            }
            Self::FailsValidation {
                code,
                field: Some(field),
            } => write!(f, "fails validation: {code} on {field}"),
            Self::WrongKind => f.write_str("kind is not level-note"),
            Self::LevelDisagreesWithLineage => {
                f.write_str("level does not match its lineage")
            }
            Self::MissingQuestion => f.write_str("question is missing"),
            Self::MalformedFollowUps => {
                f.write_str("follow_ups is not a list of questions")
            }
            Self::NotAPlainQuestion { follow_up, rule } => {
                write!(f, "follow-up {follow_up} ")?;
                match rule {
                    PlainQuestionRule::TooLong => write!(
                        f,
                        "is longer than {LONGEST_PLAIN_QUESTION} characters"
                    ),
                    PlainQuestionRule::HiddenCharacter => {
                        f.write_str("contains a hidden or control character")
                    }
                    PlainQuestionRule::Link => {
                        f.write_str("contains a link or host name")
                    }
                }
            }
            Self::QuestionDisagreesWithCandidate => {
                f.write_str("answers a different question from its node")
            }
        }
    }
}

/// The frontmatter a schema-valid level note is judged on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelNoteFields<'a> {
    pub kind: Option<&'a str>,
    pub level: Option<&'a str>,
    pub question: Option<&'a str>,
    pub follow_ups: Option<&'a [String]>,
    pub digest: Digest,
}

impl LevelNote {
    /// # Errors
    ///
    /// The first [`NoteRejection`] the fields earn.
    pub fn from_frontmatter(
        lineage: Lineage,
        fields: LevelNoteFields<'_>,
        unicode: &dyn UnicodeText,
    ) -> Result<Self, NoteRejection> {
        if fields.kind != Some(LEVEL_NOTE_KIND) {
            return Err(NoteRejection::WrongKind);
        }
        let level = fields.level.and_then(|level| level.parse::<u32>().ok());
        if level != Some(lineage.level()) {
            return Err(NoteRejection::LevelDisagreesWithLineage);
        }
        let question = fields
            .question
            .filter(|question| !question.trim().is_empty())
            .ok_or(NoteRejection::MissingQuestion)?;
        let follow_ups =
            fields.follow_ups.ok_or(NoteRejection::MalformedFollowUps)?;
        if let Some((follow_up, rule)) = first_breach(follow_ups, unicode) {
            return Err(NoteRejection::NotAPlainQuestion { follow_up, rule });
        }
        Ok(Self {
            lineage,
            question: question.to_owned(),
            follow_ups: follow_ups.to_vec(),
            digest: fields.digest,
        })
    }
}

fn first_breach(
    follow_ups: &[String],
    unicode: &dyn UnicodeText,
) -> Option<(NonZeroUsize, PlainQuestionRule)> {
    follow_ups
        .iter()
        .enumerate()
        .find_map(|(index, follow_up)| {
            let rule = plain_question_breach(follow_up, unicode)?;
            Some((NonZeroUsize::new(index + 1)?, rule))
        })
}

/// The notes read from one pair's `<stem>.levels/` directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelsDirectory {
    stem: Stem,
    root_question: Option<String>,
    notes: Vec<LevelNote>,
    rejected: BTreeMap<Lineage, NoteRejection>,
}

impl LevelsDirectory {
    /// `root_question` is the question the directory's root note names,
    /// whether or not that note counts, so the directory can hold its pair's
    /// index.
    #[must_use]
    pub const fn new(
        stem: Stem,
        root_question: Option<String>,
        notes: Vec<LevelNote>,
        rejected: BTreeMap<Lineage, NoteRejection>,
    ) -> Self {
        Self {
            stem,
            root_question,
            notes,
            rejected,
        }
    }

    #[must_use]
    pub const fn stem(&self) -> &Stem {
        &self.stem
    }

    #[must_use]
    pub fn root_question(&self) -> Option<&str> {
        self.root_question.as_deref()
    }

    #[must_use]
    pub fn notes(&self) -> &[LevelNote] {
        &self.notes
    }

    /// Each refused note's rejection, by lineage.
    #[must_use]
    pub const fn rejected(&self) -> &BTreeMap<Lineage, NoteRejection> {
        &self.rejected
    }
}

/// A node still missing its level note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub lineage: Lineage,
    pub question: String,
    pub cap: u32,
    pub known_questions: Vec<String>,
    pub rejected: Option<NoteRejection>,
}

/// A note that recorded more follow-ups than its cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trim {
    pub lineage: Lineage,
    pub recorded: u32,
    pub cap: u32,
}

impl Trim {
    #[must_use]
    pub const fn trimmed(&self) -> u32 {
        self.recorded.saturating_sub(self.cap)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeState {
    Growing(Vec<Node>),
    Complete(Vec<Lineage>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derivation {
    pub state: TreeState,
    pub trims: Vec<Trim>,
}

/// The state of the tree rooted at `pair_question`, derived level by level
/// down to `depth` from the notes in `directory`.
///
/// A level is derived only once every node above it has its note, so a
/// node's candidates never depend on which siblings finished first.
#[must_use]
pub fn derive(
    pair_question: &str,
    directory: Option<&LevelsDirectory>,
    depth: Depth,
    unicode: &dyn UnicodeText,
) -> Derivation {
    TreeWalk::new(pair_question, directory, unicode).walk(depth)
}

struct Candidate {
    lineage: Lineage,
    question: String,
}

struct TreeWalk<'a> {
    unicode: &'a dyn UnicodeText,
    pair_question: &'a str,
    notes: BTreeMap<&'a Lineage, &'a LevelNote>,
    rejected: Option<&'a BTreeMap<Lineage, NoteRejection>>,
    questions: BTreeMap<Lineage, String>,
    recorded: Vec<String>,
    trims: Vec<Trim>,
}

impl<'a> TreeWalk<'a> {
    fn new(
        pair_question: &'a str,
        directory: Option<&'a LevelsDirectory>,
        unicode: &'a dyn UnicodeText,
    ) -> Self {
        let notes = directory
            .map(|directory| {
                directory
                    .notes
                    .iter()
                    .map(|note| (&note.lineage, note))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            unicode,
            pair_question,
            notes,
            rejected: directory.map(|directory| &directory.rejected),
            questions: BTreeMap::new(),
            recorded: Vec::new(),
            trims: Vec::new(),
        }
    }

    fn walk(mut self, depth: Depth) -> Derivation {
        let mut known = BTreeSet::from([self.normalised(self.pair_question)]);
        let mut candidates = vec![Candidate {
            lineage: Lineage::root(),
            question: self.pair_question.to_owned(),
        }];
        let mut reached = Vec::new();
        for level in 1..=depth.levels() {
            if candidates.is_empty() {
                break;
            }
            for candidate in &candidates {
                known.insert(self.normalised(&candidate.question));
                self.questions.insert(
                    candidate.lineage.clone(),
                    candidate.question.clone(),
                );
            }
            let (notes, missing) = self.answer(&candidates);
            if !missing.is_empty() {
                return self.finish(TreeState::Growing(missing));
            }
            reached.extend(
                std::mem::take(&mut candidates)
                    .into_iter()
                    .map(|c| c.lineage),
            );
            if level < depth.levels() {
                candidates = self.follow_ups(&notes, level, &mut known);
            }
        }
        self.finish(TreeState::Complete(reached))
    }

    fn answer(
        &self,
        candidates: &[Candidate],
    ) -> (Vec<&'a LevelNote>, Vec<Node>) {
        let mut notes = Vec::new();
        let mut missing = Vec::new();
        for candidate in candidates {
            let wanted = self.normalised(&candidate.question);
            match self.notes.get(&candidate.lineage) {
                Some(note) if self.normalised(&note.question) == wanted => {
                    notes.push(*note);
                }
                Some(_) => missing.push(self.node(
                    candidate,
                    Some(NoteRejection::QuestionDisagreesWithCandidate),
                )),
                None => missing.push(
                    self.node(
                        candidate,
                        self.rejected
                            .and_then(|rejected| {
                                rejected.get(&candidate.lineage)
                            })
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
    ) -> Vec<Candidate> {
        let cap = follow_up_cap(level);
        let mut candidates = Vec::new();
        for note in notes {
            let recorded =
                u32::try_from(note.follow_ups.len()).unwrap_or(u32::MAX);
            if recorded > cap {
                self.trims.push(Trim {
                    lineage: note.lineage.clone(),
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
                    candidates.push(Candidate {
                        lineage: note.lineage.child(position),
                        question: follow_up.clone(),
                    });
                }
            }
        }
        candidates
    }

    fn node(
        &self,
        candidate: &Candidate,
        rejected: Option<NoteRejection>,
    ) -> Node {
        Node {
            lineage: candidate.lineage.clone(),
            question: candidate.question.clone(),
            cap: follow_up_cap(candidate.lineage.level()),
            known_questions: self.known_questions(&candidate.lineage),
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

    fn finish(self, state: TreeState) -> Derivation {
        Derivation {
            state,
            trims: self.trims,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::num::NonZeroUsize;

    use super::derive;
    use super::follow_up_cap;
    use super::Depth;
    use super::Derivation;
    use super::Digest;
    use super::LevelNote;
    use super::LevelNoteFields;
    use super::LevelsDirectory;
    use super::Node;
    use super::NoteRejection;
    use super::TreeState;
    use crate::lineage::Lineage;
    use crate::question::FakeUnicode;
    use crate::question::PlainQuestionRule;
    use crate::round::QuestionSlug;
    use crate::stem::Stem;

    const PAIR: &str = "P?";

    fn lineage(text: &str) -> Lineage {
        Lineage::parse(text).unwrap_or_else(Lineage::root)
    }

    fn depth(levels: u32) -> Depth {
        Depth::new(levels).unwrap_or_default()
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
        let stem =
            Stem::allocated(3, QuestionSlug::from(PAIR).as_str(), "web")?;
        Some(LevelsDirectory::new(
            stem,
            Some(PAIR.to_owned()),
            notes,
            rejected
                .iter()
                .map(|(at, rejection)| (lineage(at), rejection.clone()))
                .collect::<BTreeMap<_, _>>(),
        ))
    }

    fn derived(notes: Vec<LevelNote>, levels: u32) -> Derivation {
        derive(
            PAIR,
            directory(notes, &[]).as_ref(),
            depth(levels),
            &FakeUnicode,
        )
    }

    fn nodes(derivation: Derivation) -> Vec<Node> {
        match derivation.state {
            TreeState::Growing(nodes) => nodes,
            TreeState::Complete(_) => Vec::new(),
        }
    }

    fn missing(derivation: &Derivation) -> Vec<String> {
        nodes(derivation.clone())
            .iter()
            .map(|node| node.lineage.to_string())
            .collect()
    }

    fn complete(lineages: &[&str]) -> TreeState {
        TreeState::Complete(lineages.iter().map(|at| lineage(at)).collect())
    }

    fn known(derivation: &Derivation, at: &str) -> Vec<String> {
        nodes(derivation.clone())
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
    fn a_pair_with_no_notes_is_missing_its_root() {
        let nodes = nodes(derive(PAIR, None, depth(3), &FakeUnicode));
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].lineage, Lineage::root());
        assert_eq!(nodes[0].question, PAIR);
        assert_eq!(nodes[0].cap, 4);
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
            .trims
            .iter()
            .map(|t| (t.lineage.to_string(), t.recorded, t.cap, t.trimmed()))
            .collect();
        assert_eq!(
            trims,
            vec![
                ("1".to_owned(), 6, 4, 2),
                ("2-1".to_owned(), 3, 2, 1),
                ("2-2".to_owned(), 3, 2, 1),
                ("2-3".to_owned(), 3, 2, 1),
                ("2-4".to_owned(), 3, 2, 1),
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
        assert_eq!(derivation.state, complete(&["1", "2-1", "2-2"]));
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
        assert_eq!(derivation.state, complete(&["1"]));
    }

    #[test]
    fn limit_level_follow_ups_derive_nothing() {
        let derivation = derived(
            vec![note("1", PAIR, &["B"]), note("2-1", "B", &["X", "Y", "Z"])],
            2,
        );
        assert_eq!(derivation.state, complete(&["1", "2-1"]));
        assert!(derivation.trims.is_empty());
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
        assert_eq!(derivation.state, complete(&["1"]));
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
        let derivation =
            derive(PAIR, directory.as_ref(), depth(2), &FakeUnicode);
        let nodes = nodes(derivation);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].lineage, lineage("2-1"));
        assert_eq!(nodes[0].rejected, Some(rejection));
    }

    #[test]
    fn a_note_answering_a_different_question_is_missing() {
        let derivation =
            derived(vec![note("1", PAIR, &["B"]), note("2-1", "X", &[])], 2);
        let nodes = nodes(derivation);
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
        assert_eq!(derivation.state, complete(&["1", "2-2"]));
    }

    fn fields<'a>(
        kind: Option<&'a str>,
        level: Option<&'a str>,
        question: Option<&'a str>,
        follow_ups: Option<&'a [String]>,
    ) -> LevelNoteFields<'a> {
        LevelNoteFields {
            kind,
            level,
            question,
            follow_ups,
            digest: Digest::new([7; 32]),
        }
    }

    fn judged(
        at: &str,
        fields: LevelNoteFields<'_>,
    ) -> Result<(), NoteRejection> {
        LevelNote::from_frontmatter(lineage(at), fields, &FakeUnicode)
            .map(|_| ())
    }

    fn judged_follow_ups(follow_ups: &[&str]) -> Result<(), NoteRejection> {
        let owned: Vec<String> =
            follow_ups.iter().map(|&f| f.to_owned()).collect();
        judged(
            "1",
            fields(Some("level-note"), Some("1"), Some(PAIR), Some(&owned)),
        )
    }

    fn not_plain(
        position: usize,
        rule: PlainQuestionRule,
    ) -> Result<(), NoteRejection> {
        Err(NoteRejection::NotAPlainQuestion {
            follow_up: NonZeroUsize::new(position).unwrap_or(NonZeroUsize::MIN),
            rule,
        })
    }

    #[test]
    fn an_accepted_note_keeps_its_fields() {
        let follow_ups = vec!["B?".to_owned()];
        let note = LevelNote::from_frontmatter(
            lineage("2-1"),
            fields(
                Some("level-note"),
                Some("2"),
                Some("A?"),
                Some(&follow_ups),
            ),
            &FakeUnicode,
        );
        assert_eq!(
            note,
            Ok(LevelNote {
                lineage: lineage("2-1"),
                question: "A?".to_owned(),
                follow_ups,
                digest: Digest::new([7; 32]),
            })
        );
    }

    #[test]
    fn every_note_rejection_names_its_cause() {
        let none: &[String] = &[];
        let well = |at, level| {
            judged(
                at,
                fields(Some("level-note"), Some(level), Some(PAIR), Some(none)),
            )
        };
        assert_eq!(well("1", "1"), Ok(()));
        assert_eq!(
            judged(
                "1",
                fields(Some("finding"), Some("1"), Some(PAIR), Some(none))
            ),
            Err(NoteRejection::WrongKind)
        );
        assert_eq!(
            judged("1", fields(None, Some("1"), Some(PAIR), Some(none))),
            Err(NoteRejection::WrongKind)
        );
        assert_eq!(
            well("2-1", "1"),
            Err(NoteRejection::LevelDisagreesWithLineage)
        );
        assert_eq!(
            well("1", "x"),
            Err(NoteRejection::LevelDisagreesWithLineage)
        );
        assert_eq!(
            judged(
                "1",
                fields(Some("level-note"), Some("1"), None, Some(none))
            ),
            Err(NoteRejection::MissingQuestion)
        );
        assert_eq!(
            judged(
                "1",
                fields(Some("level-note"), Some("1"), Some(""), Some(none))
            ),
            Err(NoteRejection::MissingQuestion)
        );
        assert_eq!(
            judged(
                "1",
                fields(Some("level-note"), Some("1"), Some(PAIR), None)
            ),
            Err(NoteRejection::MalformedFollowUps)
        );

        let long_root = format!("See https://example.org {}", "a".repeat(300));
        assert_eq!(
            judged(
                "1",
                fields(
                    Some("level-note"),
                    Some("1"),
                    Some(&long_root),
                    Some(none)
                )
            ),
            Ok(())
        );
    }

    #[test]
    fn every_follow_up_passes_the_plain_question_gate_or_names_its_position() {
        use PlainQuestionRule::HiddenCharacter as Hidden;
        use PlainQuestionRule::Link;
        use PlainQuestionRule::TooLong;
        let rows: [(&str, Option<PlainQuestionRule>); 40] = [
            ("a\nb?", Some(Hidden)),
            ("\u{7}?", Some(Hidden)),
            ("\u{202E}?", Some(Hidden)),
            ("\u{E0041}?", Some(Hidden)),
            ("\u{FE0F}?", Some(Hidden)),
            ("\u{E0100}?", Some(Hidden)),
            ("\u{3164}?", Some(Hidden)),
            ("\u{2028}?", Some(Hidden)),
            ("\u{E000}?", Some(Hidden)),
            ("\u{0378}?", Some(Hidden)),
            ("see evil\u{FF0E}example\u{FF0F}q", Some(Link)),
            ("see evil\u{3002}example/q", Some(Link)),
            ("see evil.example:8080/q", Some(Link)),
            ("see evil.example\\q", Some(Link)),
            ("see //evil.example", Some(Link)),
            ("How does Node.js/Deno compare?", Some(Link)),
            ("What changed in Python 3.12?", None),
            ("How does it compare with Vue.js?", None),
            ("see evil.example?q=1", Some(Link)),
            ("see пример.рф/payload", Some(Link)),
            ("Что нового в релизе?", None),
            ("Is café naïve?", None),
            ("see https://x.example", Some(Link)),
            ("see www.x.example", Some(Link)),
            ("see evil.example/q", Some(Link)),
            ("How is Node.js tuned?", None),
            ("see evil.example?/x", Some(Link)),
            ("see evil.example?%41", Some(Link)),
            ("see evil%2Eexample/q", Some(Link)),
            ("Is a 1.85:1 aspect ratio better?", None),
            ("Why is a 3.5/5 rating typical?", None),
            ("see 192.168.0.1/x", Some(Link)),
            ("在这种背景下。A/B测试如何设计？", None),
            ("Vue.js和React有何不同？各自适合什么场景？", Some(Link)),
            (&"a".repeat(300), None),
            (&"a".repeat(301), Some(TooLong)),
            (&"é".repeat(300), None),
            (&"\u{FB01}".repeat(300), None),
            ("Is A?", None),
            ("", None),
        ];
        for (follow_up, rule) in rows {
            let expected = rule.map_or(Ok(()), |rule| not_plain(1, rule));
            assert_eq!(
                judged_follow_ups(&[follow_up]),
                expected,
                "{follow_up:?}"
            );
        }
        assert_eq!(
            judged_follow_ups(&["Is A?", "see https://x", "www.y"]),
            not_plain(2, Link)
        );
    }

    #[test]
    fn every_note_rejection_displays_its_reason() {
        let position = NonZeroUsize::new(2).unwrap_or(NonZeroUsize::MIN);
        let rows = [
            (
                NoteRejection::FailsValidation {
                    code: "E_X",
                    field: None,
                },
                "fails validation: E_X",
            ),
            (
                NoteRejection::FailsValidation {
                    code: "E_X",
                    field: Some("status"),
                },
                "fails validation: E_X on status",
            ),
            (NoteRejection::WrongKind, "kind is not level-note"),
            (
                NoteRejection::LevelDisagreesWithLineage,
                "level does not match its lineage",
            ),
            (NoteRejection::MissingQuestion, "question is missing"),
            (
                NoteRejection::MalformedFollowUps,
                "follow_ups is not a list of questions",
            ),
            (
                NoteRejection::NotAPlainQuestion {
                    follow_up: position,
                    rule: PlainQuestionRule::TooLong,
                },
                "follow-up 2 is longer than 300 characters",
            ),
            (
                NoteRejection::NotAPlainQuestion {
                    follow_up: position,
                    rule: PlainQuestionRule::HiddenCharacter,
                },
                "follow-up 2 contains a hidden or control character",
            ),
            (
                NoteRejection::NotAPlainQuestion {
                    follow_up: position,
                    rule: PlainQuestionRule::Link,
                },
                "follow-up 2 contains a link or host name",
            ),
            (
                NoteRejection::QuestionDisagreesWithCandidate,
                "answers a different question from its node",
            ),
        ];
        for (rejection, text) in rows {
            assert_eq!(rejection.to_string(), text);
        }
    }
}
