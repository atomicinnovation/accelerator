//! Planning a `conduct` round: which (focus area, profile) pairs an outline
//! asks for, which of them a retained finding already answers, and where each
//! outstanding pair's finding is written.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use crate::pinned_indexes::PinnedIndexes;
use crate::question::NormalisedQuestion;
use crate::question::UnicodeText;
use crate::tree::derive;
use crate::tree::Depth;
use crate::tree::Digest;
use crate::tree::LevelsDirectory;
use crate::tree::Node;
use crate::tree::NoteRef;
use crate::tree::TreeState;
use crate::tree::Trim;
use corpus::topic_research::Lineage;
use corpus::topic_research::Stem;

/// The profile an outline item is researched through when it names none.
pub const DEFAULT_PROFILE: &str = "web";

const SUFFIX_KEYWORD: &str = "profiles:";
const SUFFIX_SEPARATORS: [&str; 3] = ["—", "–", "--"];
const SLUG_MAX_LEN: usize = 60;
const EMPTY_SLUG: &str = "focus-area";

/// The checklist items of an `outline.md`, each at its one-based line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outline {
    pub items: Vec<OutlineEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineEntry {
    pub line: usize,
    pub item: OutlineItem,
}

impl Outline {
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let items = text
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                OutlineItem::parse(line).map(|item| OutlineEntry {
                    line: index + 1,
                    item,
                })
            })
            .collect();
        Self { items }
    }
}

/// One focus area: its checkbox, its question, and the profiles it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineItem {
    pub ticked: bool,
    pub question: String,
    suffix: ProfilesSuffix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProfilesSuffix {
    Absent,
    Named(Vec<String>),
    Unseparated,
}

impl OutlineItem {
    /// The item on `line`, or `None` when the line is not a `- [ ]` or
    /// `- [x]` checkbox.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let rest = line.trim_start().strip_prefix("- [")?;
        let (mark, text) = rest.split_at_checked(1)?;
        let text = text.strip_prefix(']')?;
        let ticked = match mark {
            " " => false,
            "x" | "X" => true,
            _ => return None,
        };
        let (question, suffix) = split_suffix(text.trim());
        Some(Self {
            ticked,
            question,
            suffix,
        })
    }

    #[must_use]
    pub fn profiles(&self) -> Vec<&str> {
        match &self.suffix {
            ProfilesSuffix::Named(profiles) => {
                profiles.iter().map(String::as_str).collect()
            }
            ProfilesSuffix::Absent | ProfilesSuffix::Unseparated => {
                vec![DEFAULT_PROFILE]
            }
        }
    }
}

fn split_suffix(text: &str) -> (String, ProfilesSuffix) {
    let Some(keyword_at) = text.rfind(SUFFIX_KEYWORD) else {
        return (text.to_owned(), ProfilesSuffix::Absent);
    };
    let before = text[..keyword_at].trim_end();
    let named = text[keyword_at + SUFFIX_KEYWORD.len()..]
        .split(',')
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let question = SUFFIX_SEPARATORS
        .iter()
        .find_map(|separator| before.strip_suffix(separator));
    match question {
        Some(question) if !named.is_empty() => {
            (question.trim_end().to_owned(), ProfilesSuffix::Named(named))
        }
        _ => (text.to_owned(), ProfilesSuffix::Unseparated),
    }
}

/// A question reduced to the filename-safe form its findings are named by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionSlug(String);

impl QuestionSlug {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for QuestionSlug {
    fn from(question: &str) -> Self {
        let lowered = question.to_ascii_lowercase();
        let hyphenated = lowered
            .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
            .filter(|run| !run.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let cut = hyphenated
            .get(..SLUG_MAX_LEN)
            .unwrap_or(&hyphenated)
            .trim_end_matches('-');
        Self(if cut.is_empty() {
            EMPTY_SLUG.to_owned()
        } else {
            cut.to_owned()
        })
    }
}

/// A visible `findings/*.md` file: retained when it validates, in which case
/// it answers one (question, profile) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    name: String,
    answers: Option<Answer>,
    depth: Option<Depth>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Answer {
    question: String,
    profile: String,
}

impl Finding {
    #[must_use]
    pub fn retained(name: &str, question: &str, profile: &str) -> Self {
        Self {
            name: name.to_owned(),
            answers: Some(Answer {
                question: question.to_owned(),
                profile: profile.to_owned(),
            }),
            depth: None,
        }
    }

    #[must_use]
    pub fn invalid(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            answers: None,
            depth: None,
        }
    }

    /// The finding as stamped with the depth it was researched to; an
    /// unstamped finding reads as depth 1.
    #[must_use]
    pub const fn with_depth(mut self, depth: Depth) -> Self {
        self.depth = Some(depth);
        self
    }

    fn stem(&self) -> Option<Stem> {
        self.name.strip_suffix(".md").and_then(Stem::parse)
    }
}

/// A dot-prefixed `.invalid` finding set aside by an earlier `conduct`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineMarker {
    name: String,
    question: Option<String>,
}

impl QuarantineMarker {
    #[must_use]
    pub fn new(name: &str, question: Option<String>) -> Self {
        Self {
            name: name.to_owned(),
            question,
        }
    }
}

/// Everything a round plan is derived from, read from one set.
#[derive(Debug, Clone, Default)]
pub struct RoundInputs {
    pub outline: Outline,
    pub findings: Vec<Finding>,
    pub markers: Vec<QuarantineMarker>,
    /// The brief's `source_profiles`.
    pub source_profiles: Vec<String>,
    /// The profiles with an installed `<name>-profile` skill.
    pub available_profiles: Vec<String>,
    pub levels: Vec<LevelsDirectory>,
    pub depth: Depth,
    pub pins: PinnedIndexes,
}

/// The plan for one `conduct` round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Round {
    pub items: Vec<ItemStatus>,
    pub pairs: Vec<Pair>,
    pub skipped: Vec<Skip>,
    pub warnings: Vec<Warning>,
    pub accepted_notes: BTreeMap<NoteRef, Digest>,
    /// The stems of the pairs whose finding is retained.
    pub answered: BTreeSet<Stem>,
    pub shallower: Vec<ShallowerPair>,
    pub trims: Vec<PairTrim>,
    /// The index each focus area with an outstanding pair was planned at.
    pub indexes: PinnedIndexes,
}

/// Whether an outline item is complete: it has at least one eligible pair,
/// and a retained finding answers every one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStatus {
    pub line: usize,
    pub question: String,
    pub complete: bool,
}

/// An outstanding (focus area, profile) pair and its set-relative finding
/// path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub question: String,
    pub profile: String,
    pub stem: Stem,
    pub path: String,
    pub stage: Stage,
}

impl Pair {
    #[must_use]
    pub fn level_note_path(&self, lineage: &Lineage) -> String {
        format!("findings/{}.levels/{lineage}.md", self.stem)
    }

    /// Scoped by `set_slug`, because stems repeat across sets and level-note
    /// ids must be unique across the corpus.
    #[must_use]
    pub fn level_note_id(&self, lineage: &Lineage, set_slug: &str) -> String {
        format!("{set_slug}.{}.{lineage}", self.stem)
    }
}

/// What a pair needs next: one researcher for its finding, researchers for
/// its tree's missing nodes, or the composer over its complete tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    Research,
    Deepen(Vec<Node>),
    Compose(Vec<Lineage>),
}

/// An answered pair whose finding was researched to less than the requested
/// depth, which `conduct` does not deepen again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShallowerPair {
    pub stem: Stem,
    pub depth: Depth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairTrim {
    pub stem: Stem,
    pub trim: Trim,
}

impl fmt::Display for PairTrim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "level note {}.levels/{} records {} follow-ups, over its cap of \
             {}; trimmed {}",
            self.stem,
            self.trim.lineage,
            self.trim.recorded,
            self.trim.cap,
            self.trim.trimmed()
        )
    }
}

/// A pair `conduct` cannot research.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skip {
    pub question: String,
    pub profile: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    NotInBrief,
    NotInstalled,
}

impl Skip {
    #[must_use]
    pub fn explanation(&self) -> String {
        match self.reason {
            SkipReason::NotInBrief => format!(
                "'{}' is not in the brief's source_profiles",
                self.profile
            ),
            SkipReason::NotInstalled => {
                format!("no '{}-profile' skill is installed", self.profile)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    UnseparatedProfiles { line: usize },
    UnmatchedFinding { name: String, question: String },
    IndexesExhausted { question: String },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnseparatedProfiles { line } => write!(
                f,
                "outline line {line} names 'profiles:' without a '—', '–', \
                 or '--' separator before it, so it is researched on the web"
            ),
            Self::UnmatchedFinding { name, question } => write!(
                f,
                "finding {name} answers '{question}', which no outline item \
                 asks"
            ),
            Self::IndexesExhausted { question } => write!(
                f,
                "no finding index is left to allocate to '{question}', so it \
                 is not planned"
            ),
        }
    }
}

fn index_of(name: &str) -> Option<u32> {
    let (digits, _) = name.trim_start_matches('.').split_once('-')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

impl Round {
    #[must_use]
    pub fn plan(inputs: &RoundInputs, unicode: &dyn UnicodeText) -> Self {
        Planner::new(inputs, unicode).plan()
    }
}

struct AnsweredPair<'a> {
    question: NormalisedQuestion,
    profile: &'a str,
    index: Option<u32>,
    stem: Option<Stem>,
    depth: Depth,
}

/// A quarantine marker or a `.levels` directory: a name that holds an index,
/// for the question it names when it names one.
struct IndexHolder {
    index: u32,
    question: Option<NormalisedQuestion>,
}

struct Planner<'a> {
    inputs: &'a RoundInputs,
    unicode: &'a dyn UnicodeText,
    answered: Vec<AnsweredPair<'a>>,
    holders: Vec<IndexHolder>,
    next_index: Option<u32>,
    allocated: BTreeMap<NormalisedQuestion, Option<u32>>,
}

impl<'a> Planner<'a> {
    fn new(inputs: &'a RoundInputs, unicode: &'a dyn UnicodeText) -> Self {
        let normalised =
            |question: &str| NormalisedQuestion::of(question, unicode);
        let answered: Vec<AnsweredPair<'a>> = inputs
            .findings
            .iter()
            .filter_map(|finding| {
                finding.answers.as_ref().map(|answer| AnsweredPair {
                    question: normalised(&answer.question),
                    profile: answer.profile.as_str(),
                    index: index_of(&finding.name),
                    stem: finding.stem(),
                    depth: finding.depth.unwrap_or_default(),
                })
            })
            .collect();
        let markers = inputs.markers.iter().filter_map(|marker| {
            Some(IndexHolder {
                index: index_of(&marker.name)?,
                question: marker.question.as_deref().map(normalised),
            })
        });
        let levels = inputs.levels.iter().map(|directory| IndexHolder {
            index: directory.stem().index(),
            question: directory.root_question().map(normalised),
        });
        let holders: Vec<IndexHolder> = markers.chain(levels).collect();
        let highest = inputs
            .findings
            .iter()
            .filter_map(|finding| index_of(&finding.name))
            .chain(holders.iter().map(|holder| holder.index))
            .chain(inputs.pins.indexes())
            .max();
        Self {
            inputs,
            unicode,
            answered,
            holders,
            next_index: highest.map_or(Some(1), |index| index.checked_add(1)),
            allocated: BTreeMap::new(),
        }
    }

    fn normalised(&self, question: &str) -> NormalisedQuestion {
        NormalisedQuestion::of(question, self.unicode)
    }

    fn plan(mut self) -> Round {
        let mut round = Round {
            items: Vec::new(),
            pairs: Vec::new(),
            skipped: Vec::new(),
            warnings: Vec::new(),
            accepted_notes: self.accepted_notes(),
            answered: self
                .answered
                .iter()
                .filter_map(|pair| pair.stem.clone())
                .collect(),
            shallower: Vec::new(),
            trims: Vec::new(),
            indexes: PinnedIndexes::default(),
        };
        for entry in &self.inputs.outline.items {
            if entry.item.suffix == ProfilesSuffix::Unseparated {
                round
                    .warnings
                    .push(Warning::UnseparatedProfiles { line: entry.line });
            }
            let complete = self.plan_item(&entry.item, &mut round);
            round.items.push(ItemStatus {
                line: entry.line,
                question: entry.item.question.clone(),
                complete,
            });
        }
        round.warnings.extend(self.unmatched_findings());
        for (question, index) in &self.allocated {
            if let Some(index) = index {
                round.indexes.pin(question.clone(), *index);
            }
        }
        round
    }

    fn accepted_notes(&self) -> BTreeMap<NoteRef, Digest> {
        self.inputs
            .levels
            .iter()
            .flat_map(|directory| {
                directory.notes().iter().map(|note| {
                    let at = NoteRef {
                        stem: directory.stem().clone(),
                        lineage: note.lineage.clone(),
                    };
                    (at, note.digest)
                })
            })
            .collect()
    }

    fn plan_item(&mut self, item: &OutlineItem, round: &mut Round) -> bool {
        let question = self.normalised(&item.question);
        let first_sighting = !self.allocated.contains_key(&question);
        let mut eligible = 0;
        let mut outstanding = Vec::new();
        for profile in item.profiles() {
            if let Some(reason) = self.ineligibility(profile) {
                if first_sighting {
                    round.skipped.push(Skip {
                        question: item.question.clone(),
                        profile: profile.to_owned(),
                        reason,
                    });
                }
                continue;
            }
            eligible += 1;
            match self.answer(&question, profile) {
                Some(answer) if first_sighting => {
                    round.shallower.extend(self.shallower(answer));
                }
                Some(_) => {}
                None => outstanding.push(profile),
            }
        }
        if first_sighting {
            let index = if outstanding.is_empty() {
                None
            } else {
                self.index_for(&question)
            };
            if !outstanding.is_empty() && index.is_none() {
                round.warnings.push(Warning::IndexesExhausted {
                    question: item.question.clone(),
                });
            }
            self.allocated.insert(question, index);
            if let Some(index) = index {
                let slug = QuestionSlug::from(item.question.as_str());
                for profile in outstanding.iter().copied() {
                    let Some(stem) =
                        Stem::allocated(index, slug.as_str(), profile)
                    else {
                        continue;
                    };
                    let stage = self.stage_for(&stem, &item.question, round);
                    round.pairs.push(Pair {
                        question: item.question.clone(),
                        profile: profile.to_owned(),
                        path: format!("findings/{stem}.md"),
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
        round: &mut Round,
    ) -> Stage {
        let directory = self
            .inputs
            .levels
            .iter()
            .find(|directory| directory.stem() == stem);
        if directory.is_none() && self.inputs.depth == Depth::default() {
            return Stage::Research;
        }
        let derivation =
            derive(question, directory, self.inputs.depth, self.unicode);
        round
            .trims
            .extend(derivation.trims.into_iter().map(|trim| PairTrim {
                stem: stem.clone(),
                trim,
            }));
        match derivation.state {
            TreeState::Growing(nodes) => Stage::Deepen(nodes),
            TreeState::Complete(lineages) => Stage::Compose(lineages),
        }
    }

    fn shallower(&self, answer: &AnsweredPair<'_>) -> Option<ShallowerPair> {
        (answer.depth < self.inputs.depth).then(|| {
            answer.stem.clone().map(|stem| ShallowerPair {
                stem,
                depth: answer.depth,
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
    ) -> Option<&AnsweredPair<'a>> {
        self.answered
            .iter()
            .find(|pair| pair.question == *question && pair.profile == profile)
    }

    fn index_for(&mut self, question: &NormalisedQuestion) -> Option<u32> {
        let retained = || {
            self.answered
                .iter()
                .filter(|pair| pair.question == *question)
                .filter_map(|pair| pair.index)
                .min()
        };
        let held = || {
            self.holders
                .iter()
                .filter(|holder| holder.question.as_ref() == Some(question))
                .map(|holder| holder.index)
                .min()
        };
        self.inputs
            .pins
            .index_for(question)
            .or_else(retained)
            .or_else(held)
            .or_else(|| {
                let index = self.next_index?;
                self.next_index = index.checked_add(1);
                Some(index)
            })
    }

    fn unmatched_findings(&self) -> Vec<Warning> {
        let asked: Vec<NormalisedQuestion> = self
            .inputs
            .outline
            .items
            .iter()
            .map(|entry| self.normalised(&entry.item.question))
            .collect();
        self.inputs
            .findings
            .iter()
            .filter_map(|finding| {
                let answer = finding.answers.as_ref()?;
                (!asked.contains(&self.normalised(&answer.question))).then(
                    || Warning::UnmatchedFinding {
                        name: finding.name.clone(),
                        question: answer.question.clone(),
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::Finding;
    use super::Outline;
    use super::OutlineItem;
    use super::Pair;
    use super::PairTrim;
    use super::QuarantineMarker;
    use super::QuestionSlug;
    use super::Round;
    use super::RoundInputs;
    use super::ShallowerPair;
    use super::Skip;
    use super::SkipReason;
    use super::Stage;
    use super::Warning;
    use crate::pinned_indexes::PinnedIndexes;
    use crate::question::FakeUnicode;
    use crate::question::NormalisedQuestion;
    use crate::tree::Depth;
    use crate::tree::Digest;
    use crate::tree::LevelNote;
    use crate::tree::LevelsDirectory;
    use crate::tree::NoteRef;
    use crate::tree::NoteRejection;
    use crate::tree::Trim;
    use corpus::topic_research::Lineage;
    use corpus::topic_research::Stem;

    const DIGEST: Digest = Digest::new([9; 32]);

    fn plan(inputs: &RoundInputs) -> Round {
        Round::plan(inputs, &FakeUnicode)
    }

    fn lineage(text: &str) -> Lineage {
        Lineage::parse(text).unwrap_or_else(Lineage::root)
    }

    fn depth(levels: u32) -> Depth {
        Depth::new(levels).unwrap_or_default()
    }

    fn stems(round: &Round) -> Vec<String> {
        round
            .pairs
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

    fn pins(entries: &[(&str, u32)]) -> PinnedIndexes {
        let mut pins = PinnedIndexes::default();
        for (question, index) in entries {
            pins.pin(NormalisedQuestion::of(question, &FakeUnicode), *index);
        }
        pins
    }

    fn stage(round: &Round) -> Option<&Stage> {
        round.pairs.first().map(|pair| &pair.stage)
    }

    fn deepened(round: &Round) -> Vec<String> {
        match stage(round) {
            Some(Stage::Deepen(nodes)) => {
                nodes.iter().map(|node| node.lineage.to_string()).collect()
            }
            _ => Vec::new(),
        }
    }

    fn outline(lines: &[&str]) -> Outline {
        Outline::parse(&lines.join("\n"))
    }

    fn retained(name: &str, question: &str, profile: &str) -> Finding {
        Finding::retained(name, question, profile)
    }

    fn inputs(outline: Outline) -> RoundInputs {
        RoundInputs {
            outline,
            findings: Vec::new(),
            markers: Vec::new(),
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
            pins: PinnedIndexes::default(),
        }
    }

    fn paths(round: &Round) -> Vec<(&str, &str)> {
        round
            .pairs
            .iter()
            .map(|pair| (pair.profile.as_str(), pair.path.as_str()))
            .collect()
    }

    #[test]
    fn an_item_without_a_suffix_is_researched_on_the_web(
    ) -> Result<(), &'static str> {
        let item = OutlineItem::parse("- [ ] How do heads specialise?")
            .ok_or("an item")?;
        assert!(!item.ticked);
        assert_eq!(item.question, "How do heads specialise?");
        assert_eq!(item.profiles(), vec!["web"]);
        Ok(())
    }

    #[test]
    fn each_recognised_separator_introduces_the_profiles_suffix(
    ) -> Result<(), &'static str> {
        for separator in ["—", "–", "--"] {
            let line =
                format!("- [x] Why? {separator} profiles: web, openalex");
            let item = OutlineItem::parse(&line).ok_or("an item")?;
            assert!(item.ticked);
            assert_eq!(item.question, "Why?");
            assert_eq!(item.profiles(), vec!["web", "openalex"]);
        }
        Ok(())
    }

    #[test]
    fn a_line_that_is_not_a_checkbox_is_not_an_item() {
        assert!(OutlineItem::parse("## Round 1").is_none());
        assert!(OutlineItem::parse("- plain bullet").is_none());
        assert!(OutlineItem::parse("").is_none());
    }

    #[test]
    fn an_unseparated_profiles_suffix_is_a_warning_and_defaults_to_web() {
        let outline = outline(&["# O", "- [ ] Why? profiles: arxiv"]);
        let round = plan(&inputs(outline));
        assert_eq!(round.items[0].question, "Why? profiles: arxiv");
        assert_eq!(
            paths(&round),
            vec![("web", "findings/01-why-profiles-arxiv-web.md")]
        );
        assert_eq!(round.warnings.len(), 1);
        assert!(
            round.warnings[0].to_string().contains("line 2"),
            "{:?}",
            round.warnings
        );
    }

    #[test]
    fn items_carry_their_one_based_line_number() {
        let outline = outline(&["---", "kind: outline", "---", "- [ ] A?"]);
        assert_eq!(outline.items[0].line, 4);
    }

    #[test]
    fn a_question_slug_keeps_ascii_alphanumerics_and_hyphenates_the_rest() {
        assert_eq!(
            QuestionSlug::from("How do attention heads specialise?").as_str(),
            "how-do-attention-heads-specialise"
        );
        assert_eq!(QuestionSlug::from("  --Why?? ").as_str(), "why");
    }

    #[test]
    fn a_question_with_no_ascii_alphanumerics_slugs_to_focus_area() {
        assert_eq!(QuestionSlug::from("注意力机制？").as_str(), "focus-area");
        assert_eq!(QuestionSlug::from("").as_str(), "focus-area");
    }

    #[test]
    fn a_question_slug_is_cut_to_sixty_characters() {
        let sixty = "a".repeat(60);
        assert_eq!(QuestionSlug::from(sixty.as_str()).as_str(), sixty);
        let sixty_one = "b".repeat(61);
        assert_eq!(
            QuestionSlug::from(sixty_one.as_str()).as_str(),
            "b".repeat(60)
        );
    }

    #[test]
    fn a_cut_never_leaves_a_trailing_hyphen() {
        let question = format!("{} tail", "c".repeat(59));
        assert_eq!(
            QuestionSlug::from(question.as_str()).as_str(),
            "c".repeat(59)
        );
    }

    #[test]
    fn a_fresh_multi_profile_item_shares_one_index() {
        let round =
            plan(&inputs(outline(&["- [ ] How? — profiles: web, openalex"])));
        assert_eq!(
            paths(&round),
            vec![
                ("web", "findings/01-how-web.md"),
                ("openalex", "findings/01-how-openalex.md"),
            ]
        );
        assert!(!round.items[0].complete);
    }

    #[test]
    fn two_fresh_items_take_consecutive_indices() {
        let round = plan(&inputs(outline(&["- [ ] A?", "- [ ] B?"])));
        assert_eq!(
            paths(&round),
            vec![
                ("web", "findings/01-a-web.md"),
                ("web", "findings/02-b-web.md")
            ]
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
        let round = plan(&inputs);
        assert_eq!(
            paths(&round),
            vec![("openalex", "findings/03-b-openalex.md")]
        );
        assert!(round.items[0].complete);
        assert!(!round.items[1].complete);
    }

    #[test]
    fn a_quarantined_pair_reuses_its_markers_index() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] C?"]));
        inputs.findings = vec![retained("01-a-web.md", "A?", "web")];
        inputs.markers = vec![QuarantineMarker::new(
            ".04-c-web.md.invalid",
            Some("C?".into()),
        )];
        let round = plan(&inputs);
        assert_eq!(paths(&round), vec![("web", "findings/04-c-web.md")]);
    }

    #[test]
    fn a_new_focus_area_never_reuses_an_index_a_marker_holds() {
        let mut inputs = inputs(outline(&["- [ ] New?"]));
        inputs.markers =
            vec![QuarantineMarker::new(".02-old-web.md.invalid", None)];
        let round = plan(&inputs);
        assert_eq!(paths(&round), vec![("web", "findings/03-new-web.md")]);
    }

    #[test]
    fn an_invalid_finding_holds_its_index_but_leaves_its_pair_outstanding() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] B?"]));
        inputs.findings = vec![Finding::invalid("01-a-web.md")];
        let round = plan(&inputs);
        assert_eq!(
            paths(&round),
            vec![
                ("web", "findings/02-a-web.md"),
                ("web", "findings/03-b-web.md")
            ]
        );
    }

    #[test]
    fn a_profile_outside_the_brief_is_skipped() {
        let mut inputs = inputs(outline(&["- [ ] A? — profiles: web, arxiv"]));
        inputs.source_profiles = vec!["web".into()];
        let round = plan(&inputs);
        assert_eq!(paths(&round), vec![("web", "findings/01-a-web.md")]);
        assert_eq!(
            round.skipped,
            vec![Skip {
                question: "A?".into(),
                profile: "arxiv".into(),
                reason: SkipReason::NotInBrief,
            }]
        );
        assert_eq!(
            round.skipped[0].explanation(),
            "'arxiv' is not in the brief's source_profiles"
        );
    }

    #[test]
    fn a_profile_with_no_skill_is_skipped() {
        let mut inputs = inputs(outline(&["- [ ] A? — profiles: crossref"]));
        inputs.source_profiles.push("crossref".into());
        let round = plan(&inputs);
        assert!(round.pairs.is_empty());
        assert_eq!(
            round.skipped[0].explanation(),
            "no 'crossref-profile' skill is installed"
        );
    }

    #[test]
    fn an_item_whose_pairs_are_all_skipped_is_never_complete() {
        let mut inputs = inputs(outline(&["- [x] A? — profiles: arxiv"]));
        inputs.source_profiles = vec!["web".into()];
        let round = plan(&inputs);
        assert!(!round.items[0].complete);
    }

    #[test]
    fn a_skipped_pair_does_not_hold_back_an_otherwise_complete_item() {
        let mut inputs = inputs(outline(&["- [ ] A? — profiles: web, arxiv"]));
        inputs.source_profiles = vec!["web".into()];
        inputs.findings = vec![retained("01-a-web.md", "A?", "web")];
        let round = plan(&inputs);
        assert!(round.items[0].complete);
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
        let round = plan(&inputs);
        assert!(round.pairs.is_empty());
        assert!(round.items[0].complete);
        assert!(round.warnings.is_empty());
    }

    #[test]
    fn a_non_latin_question_is_allocated_a_focus_area_slug() {
        let round = plan(&inputs(outline(&["- [ ] 注意力机制？"])));
        assert_eq!(
            paths(&round),
            vec![("web", "findings/01-focus-area-web.md")]
        );
    }

    #[test]
    fn questions_match_after_collapsing_whitespace() {
        let mut inputs =
            inputs(outline(&["- [ ] How   do heads\tspecialise?"]));
        inputs.findings =
            vec![retained("01-x.md", " How do heads specialise? ", "web")];
        let round = plan(&inputs);
        assert!(round.items[0].complete);
        assert!(round.warnings.is_empty());
    }

    #[test]
    fn a_finding_answering_no_outline_question_is_a_warning() {
        let mut inputs = inputs(outline(&["- [ ] How do heads specialise?"]));
        inputs.findings =
            vec![retained("01-x.md", "How do the heads specialise?", "web")];
        let round = plan(&inputs);
        assert!(!round.items[0].complete);
        assert_eq!(
            paths(&round),
            vec![("web", "findings/02-how-do-heads-specialise-web.md")]
        );
        assert_eq!(round.warnings.len(), 1);
        assert!(
            round.warnings[0].to_string().contains("01-x.md"),
            "{:?}",
            round.warnings
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
        let round = plan(&inputs);
        assert_eq!(paths(&round), vec![("arxiv", "findings/02-a-arxiv.md")]);
    }

    #[test]
    fn a_ticked_item_that_is_newly_incomplete_reports_incomplete() {
        let round =
            plan(&inputs(outline(&["- [x] A? — profiles: web, openalex"])));
        assert!(!round.items[0].complete);
        assert_eq!(round.pairs.len(), 2);
    }

    #[test]
    fn a_question_repeated_across_rounds_is_allocated_once() {
        let round = plan(&inputs(outline(&["- [ ] A?", "- [ ] A?"])));
        assert_eq!(round.items.len(), 2);
        assert_eq!(
            round.pairs,
            Stem::parse("01-a-web")
                .map(|stem| Pair {
                    question: "A?".into(),
                    profile: "web".into(),
                    stem,
                    path: "findings/01-a-web.md".into(),
                    stage: Stage::Research,
                })
                .into_iter()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_pair_without_levels_at_depth_one_is_researched_directly() {
        let round = plan(&inputs(outline(&["- [ ] A?"])));
        assert_eq!(stems(&round), ["01-a-web"]);
        assert_eq!(stage(&round), Some(&Stage::Research));
        assert!(round.trims.is_empty());
        assert!(round.shallower.is_empty());
    }

    #[test]
    fn a_pair_at_depth_two_with_no_levels_deepens_from_its_root() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(2);
        let round = plan(&inputs);
        assert_eq!(stems(&round), ["01-a-web"]);
        assert_eq!(deepened(&round), ["1"]);
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
        let round = plan(&inputs);
        assert_eq!(
            stage(&round),
            Some(&Stage::Compose(vec![lineage("1"), lineage("2-1")]))
        );
    }

    #[test]
    fn a_pair_at_depth_one_with_a_root_note_composes_from_it() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.levels =
            levels("01-a-web", Some("A?"), vec![note("1", "A?", &["B?"])]);
        let round = plan(&inputs);
        assert_eq!(stage(&round), Some(&Stage::Compose(vec![lineage("1")])));
    }

    #[test]
    fn a_levels_directory_without_a_root_at_depth_one_deepens_its_root() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.levels = levels("01-a-web", Some("A?"), Vec::new());
        let round = plan(&inputs);
        assert_eq!(stems(&round), ["01-a-web"]);
        assert_eq!(deepened(&round), ["1"]);
    }

    #[test]
    fn a_levels_directory_holds_its_index_through_its_root_note() {
        let mut inputs = inputs(outline(&["- [ ] Z?", "- [ ] A?"]));
        inputs.levels =
            levels("04-a-web", Some("A?"), vec![note("1", "A?", &[])]);
        let round = plan(&inputs);
        assert_eq!(stems(&round), ["05-z-web", "04-a-web"]);
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
        let round = plan(&inputs);
        assert_eq!(stems(&round), ["04-a-web"]);
        let Some(Stage::Deepen(nodes)) = stage(&round) else {
            return assert_eq!(stage(&round), None, "a deepen stage");
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
        inputs.markers = vec![QuarantineMarker::new(
            ".06-a-web.md.invalid",
            Some("A?".into()),
        )];
        inputs.levels = levels("04-a-openalex", Some("A?"), Vec::new());
        assert_eq!(stems(&plan(&inputs)), ["04-a-web", "04-a-openalex"]);
        inputs.markers = vec![QuarantineMarker::new(
            ".02-a-web.md.invalid",
            Some("A?".into()),
        )];
        assert_eq!(stems(&plan(&inputs)), ["02-a-web", "02-a-openalex"]);
    }

    #[test]
    fn an_index_at_the_ceiling_never_overflows_allocation() {
        let mut inputs = inputs(outline(&["- [ ] M?", "- [ ] N?"]));
        inputs.levels = levels("4294967295-x-web", None, Vec::new());
        let round = plan(&inputs);
        assert!(round.pairs.is_empty());
        assert_eq!(
            round.warnings,
            vec![
                Warning::IndexesExhausted {
                    question: "M?".into()
                },
                Warning::IndexesExhausted {
                    question: "N?".into()
                },
            ]
        );
        assert!(round.warnings[0].to_string().contains("'M?'"));

        inputs.levels = levels("4294967294-x-web", None, Vec::new());
        let round = plan(&inputs);
        assert_eq!(stems(&round), ["4294967295-m-web"]);
        assert_eq!(
            round.warnings,
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
        let round = plan(&inputs);
        assert!(round.pairs.is_empty());
        assert!(round.items[0].complete);
    }

    #[test]
    fn a_retained_finding_below_the_requested_depth_is_reported_shallower() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(3);
        inputs.findings =
            vec![retained("01-a-web.md", "A?", "web").with_depth(depth(2))];
        let round = plan(&inputs);
        assert_eq!(
            round.shallower,
            Stem::parse("01-a-web")
                .map(|stem| ShallowerPair {
                    stem,
                    depth: depth(2)
                })
                .into_iter()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_legacy_finding_is_reported_shallower_than_depth_two() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(2);
        inputs.findings = vec![retained("01-a-web.md", "A?", "web")];
        let round = plan(&inputs);
        assert_eq!(round.shallower.len(), 1);
        assert_eq!(round.shallower[0].depth, Depth::default());
        inputs.depth = Depth::default();
        assert!(plan(&inputs).shallower.is_empty());
    }

    #[test]
    fn a_retained_finding_at_the_requested_depth_is_not_reported_shallower() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] A?"]));
        inputs.depth = depth(3);
        inputs.findings =
            vec![retained("01-a-web.md", "A?", "web").with_depth(depth(3))];
        assert!(plan(&inputs).shallower.is_empty());
        inputs.findings =
            vec![retained("01-a-web.md", "A?", "web").with_depth(depth(1))];
        assert_eq!(plan(&inputs).shallower.len(), 1);
    }

    #[test]
    fn a_failed_pairs_stem_survives_a_later_pair_writing_within_a_run() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] B?"]));
        inputs.findings = vec![retained("02-b-web.md", "B?", "web")];
        assert_eq!(stems(&plan(&inputs)), ["03-a-web"]);
        inputs.pins = pins(&[("A?", 1), ("B?", 2)]);
        assert_eq!(stems(&plan(&inputs)), ["01-a-web"]);
    }

    #[test]
    fn a_pair_keeps_its_stem_when_its_root_note_records_another_question() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(2);
        inputs.levels =
            levels("01-a-web", Some("Other?"), vec![note("1", "Other?", &[])]);
        assert_eq!(stems(&plan(&inputs)), ["02-a-web"]);
        inputs.pins = pins(&[("A?", 1)]);
        let round = plan(&inputs);
        assert_eq!(stems(&round), ["01-a-web"]);
        let Some(Stage::Deepen(nodes)) = stage(&round) else {
            return assert_eq!(stage(&round), None, "a deepen stage");
        };
        assert_eq!(
            nodes[0].rejected,
            Some(NoteRejection::QuestionDisagreesWithCandidate)
        );
    }

    #[test]
    fn a_new_focus_area_never_takes_a_pinned_index() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] New?"]));
        inputs.pins = pins(&[("A?", 5)]);
        assert_eq!(stems(&plan(&inputs)), ["05-a-web", "06-new-web"]);
    }

    #[test]
    fn a_round_records_the_index_each_planned_focus_area_holds() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] B?", "- [ ] C?"]));
        inputs.findings = vec![retained("02-b-web.md", "B?", "web")];
        let round = plan(&inputs);
        assert_eq!(round.indexes, pins(&[("A?", 3), ("C?", 4)]));
    }

    #[test]
    fn a_round_lists_its_accepted_notes_and_answered_stems() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] B?"]));
        inputs.findings = vec![
            retained("02-b-web.md", "B?", "web"),
            Finding::invalid("03-c-web.md"),
        ];
        inputs.levels =
            levels("01-a-web", Some("A?"), vec![note("1", "A?", &["X?"])]);
        let round = plan(&inputs);
        let accepted: Vec<(String, Digest)> = round
            .accepted_notes
            .iter()
            .map(|(at, digest): (&NoteRef, &Digest)| (at.to_string(), *digest))
            .collect();
        assert_eq!(accepted, vec![("01-a-web:1".to_owned(), DIGEST)]);
        let answered: Vec<String> =
            round.answered.iter().map(ToString::to_string).collect();
        assert_eq!(answered, ["02-b-web"]);
    }

    #[test]
    fn trims_name_the_pair_stem_and_lineage() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(2);
        inputs.levels = levels(
            "01-a-web",
            Some("A?"),
            vec![note("1", "A?", &["B?", "C?", "D?", "E?", "F?"])],
        );
        let round = plan(&inputs);
        assert_eq!(
            round.trims,
            Stem::parse("01-a-web")
                .map(|stem| PairTrim {
                    stem,
                    trim: Trim {
                        lineage: Lineage::root(),
                        recorded: 5,
                        cap: 4
                    },
                })
                .into_iter()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            round.trims[0].to_string(),
            "level note 01-a-web.levels/1 records 5 follow-ups, over its cap \
             of 4; trimmed 1"
        );
    }

    #[test]
    fn known_questions_render_candidate_text_not_a_notes_own_question() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.depth = depth(3);
        inputs.levels =
            levels("01-a-web", Some("A?"), vec![note("1", "  A?\n", &["B?"])]);
        let round = plan(&inputs);
        let Some(Stage::Deepen(nodes)) = stage(&round) else {
            return assert_eq!(stage(&round), None, "a deepen stage");
        };
        assert_eq!(nodes[0].lineage, lineage("2-1"));
        assert_eq!(nodes[0].known_questions, ["A?", "B?"]);
    }

    #[test]
    fn a_finding_answers_an_outline_item_equal_under_compatibility_folding() {
        let mut inputs = inputs(outline(&["- [ ] A?"]));
        inputs.findings = vec![retained("01-a-web.md", "A\u{FF1F}", "web")];
        let round = plan(&inputs);
        assert!(round.items[0].complete);
        assert!(round.pairs.is_empty());
        assert!(round.warnings.is_empty());
    }

    #[test]
    fn a_pairs_level_note_path_and_id_share_its_stem_and_lineage() {
        let round = plan(&inputs(outline(&["- [ ] A?"])));
        let at = lineage("3-2-1");
        let paths: Vec<(String, String)> = round
            .pairs
            .iter()
            .map(|pair| {
                (
                    pair.level_note_path(&at),
                    pair.level_note_id(&at, "attention"),
                )
            })
            .collect();
        assert_eq!(
            paths,
            vec![(
                "findings/01-a-web.levels/3-2-1.md".to_owned(),
                "attention.01-a-web.3-2-1".to_owned()
            )]
        );
    }
}
