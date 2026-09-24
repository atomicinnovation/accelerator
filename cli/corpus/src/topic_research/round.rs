//! Planning a `conduct` round: which (focus area, profile) pairs an outline
//! asks for, which of them a retained finding already answers, and where each
//! outstanding pair's finding is written.

use std::collections::BTreeMap;
use std::fmt;

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
        }
    }

    #[must_use]
    pub fn invalid(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            answers: None,
        }
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
}

/// The plan for one `conduct` round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Round {
    pub items: Vec<ItemStatus>,
    pub pairs: Vec<Pair>,
    pub skipped: Vec<Skip>,
    pub warnings: Vec<Warning>,
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
    pub path: String,
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
        }
    }
}

/// Questions compare after trimming and collapsing runs of whitespace, so a
/// reflowed outline still matches its findings.
fn normalised(question: &str) -> String {
    question.split_whitespace().collect::<Vec<_>>().join(" ")
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
    pub fn plan(inputs: &RoundInputs) -> Self {
        Planner::new(inputs).plan()
    }
}

struct AnsweredPair<'a> {
    question: String,
    profile: &'a str,
    index: Option<u32>,
}

struct Planner<'a> {
    inputs: &'a RoundInputs,
    answered: Vec<AnsweredPair<'a>>,
    next_index: u32,
    allocated: BTreeMap<String, Option<u32>>,
}

impl<'a> Planner<'a> {
    fn new(inputs: &'a RoundInputs) -> Self {
        let answered = inputs
            .findings
            .iter()
            .filter_map(|finding| {
                finding.answers.as_ref().map(|answer| AnsweredPair {
                    question: normalised(&answer.question),
                    profile: answer.profile.as_str(),
                    index: index_of(&finding.name),
                })
            })
            .collect();
        let highest = inputs
            .findings
            .iter()
            .map(|finding| &finding.name)
            .chain(inputs.markers.iter().map(|marker| &marker.name))
            .filter_map(|name| index_of(name))
            .max()
            .unwrap_or(0);
        Self {
            inputs,
            answered,
            next_index: highest + 1,
            allocated: BTreeMap::new(),
        }
    }

    fn plan(mut self) -> Round {
        let mut round = Round {
            items: Vec::new(),
            pairs: Vec::new(),
            skipped: Vec::new(),
            warnings: Vec::new(),
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
        round
    }

    fn plan_item(&mut self, item: &OutlineItem, round: &mut Round) -> bool {
        let question = normalised(&item.question);
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
            if !self.is_answered(&question, profile) {
                outstanding.push(profile);
            }
        }
        if first_sighting {
            let index =
                (!outstanding.is_empty()).then(|| self.index_for(&question));
            self.allocated.insert(question, index);
            if let Some(index) = index {
                let slug = QuestionSlug::from(item.question.as_str());
                round.pairs.extend(outstanding.iter().map(|profile| Pair {
                    question: item.question.clone(),
                    profile: (*profile).to_owned(),
                    path: format!(
                        "findings/{index:02}-{}-{profile}.md",
                        slug.as_str()
                    ),
                }));
            }
        }
        eligible > 0 && outstanding.is_empty()
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

    fn is_answered(&self, question: &str, profile: &str) -> bool {
        self.answered
            .iter()
            .any(|pair| pair.question == question && pair.profile == profile)
    }

    fn index_for(&mut self, question: &str) -> u32 {
        let retained = self
            .answered
            .iter()
            .filter(|pair| pair.question == question)
            .filter_map(|pair| pair.index)
            .min();
        let quarantined = || {
            self.inputs
                .markers
                .iter()
                .filter(|marker| {
                    marker
                        .question
                        .as_deref()
                        .is_some_and(|named| normalised(named) == question)
                })
                .filter_map(|marker| index_of(&marker.name))
                .min()
        };
        retained.or_else(quarantined).unwrap_or_else(|| {
            let index = self.next_index;
            self.next_index += 1;
            index
        })
    }

    fn unmatched_findings(&self) -> Vec<Warning> {
        let asked: Vec<String> = self
            .inputs
            .outline
            .items
            .iter()
            .map(|entry| normalised(&entry.item.question))
            .collect();
        self.inputs
            .findings
            .iter()
            .filter_map(|finding| {
                let answer = finding.answers.as_ref()?;
                (!asked.contains(&normalised(&answer.question))).then(|| {
                    Warning::UnmatchedFinding {
                        name: finding.name.clone(),
                        question: answer.question.clone(),
                    }
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Finding;
    use super::Outline;
    use super::OutlineItem;
    use super::Pair;
    use super::QuarantineMarker;
    use super::QuestionSlug;
    use super::Round;
    use super::RoundInputs;
    use super::Skip;
    use super::SkipReason;

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
        let round = Round::plan(&inputs(outline));
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
        let round = Round::plan(&inputs(outline(&[
            "- [ ] How? — profiles: web, openalex",
        ])));
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
        let round = Round::plan(&inputs(outline(&["- [ ] A?", "- [ ] B?"])));
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
        let round = Round::plan(&inputs);
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
        let round = Round::plan(&inputs);
        assert_eq!(paths(&round), vec![("web", "findings/04-c-web.md")]);
    }

    #[test]
    fn a_new_focus_area_never_reuses_an_index_a_marker_holds() {
        let mut inputs = inputs(outline(&["- [ ] New?"]));
        inputs.markers =
            vec![QuarantineMarker::new(".02-old-web.md.invalid", None)];
        let round = Round::plan(&inputs);
        assert_eq!(paths(&round), vec![("web", "findings/03-new-web.md")]);
    }

    #[test]
    fn an_invalid_finding_holds_its_index_but_leaves_its_pair_outstanding() {
        let mut inputs = inputs(outline(&["- [ ] A?", "- [ ] B?"]));
        inputs.findings = vec![Finding::invalid("01-a-web.md")];
        let round = Round::plan(&inputs);
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
        let round = Round::plan(&inputs);
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
        let round = Round::plan(&inputs);
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
        let round = Round::plan(&inputs);
        assert!(!round.items[0].complete);
    }

    #[test]
    fn a_skipped_pair_does_not_hold_back_an_otherwise_complete_item() {
        let mut inputs = inputs(outline(&["- [ ] A? — profiles: web, arxiv"]));
        inputs.source_profiles = vec!["web".into()];
        inputs.findings = vec![retained("01-a-web.md", "A?", "web")];
        let round = Round::plan(&inputs);
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
        let round = Round::plan(&inputs);
        assert!(round.pairs.is_empty());
        assert!(round.items[0].complete);
        assert!(round.warnings.is_empty());
    }

    #[test]
    fn a_non_latin_question_is_allocated_a_focus_area_slug() {
        let round = Round::plan(&inputs(outline(&["- [ ] 注意力机制？"])));
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
        let round = Round::plan(&inputs);
        assert!(round.items[0].complete);
        assert!(round.warnings.is_empty());
    }

    #[test]
    fn a_finding_answering_no_outline_question_is_a_warning() {
        let mut inputs = inputs(outline(&["- [ ] How do heads specialise?"]));
        inputs.findings =
            vec![retained("01-x.md", "How do the heads specialise?", "web")];
        let round = Round::plan(&inputs);
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
        let round = Round::plan(&inputs);
        assert_eq!(paths(&round), vec![("arxiv", "findings/02-a-arxiv.md")]);
    }

    #[test]
    fn a_ticked_item_that_is_newly_incomplete_reports_incomplete() {
        let round = Round::plan(&inputs(outline(&[
            "- [x] A? — profiles: web, openalex",
        ])));
        assert!(!round.items[0].complete);
        assert_eq!(round.pairs.len(), 2);
    }

    #[test]
    fn a_question_repeated_across_rounds_is_allocated_once() {
        let round = Round::plan(&inputs(outline(&["- [ ] A?", "- [ ] A?"])));
        assert_eq!(round.items.len(), 2);
        assert_eq!(
            round.pairs,
            vec![Pair {
                question: "A?".into(),
                profile: "web".into(),
                path: "findings/01-a-web.md".into(),
            }]
        );
    }
}
