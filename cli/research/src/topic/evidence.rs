//! What a set's `findings/` directory shows: its findings, and each pair's
//! level notes with the reason any of them does not count.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use crate::topic::claims::IndexClaim;
use crate::topic::layout::lineage::Lineage;
use crate::topic::layout::stem::Stem;
use crate::topic::outline::Pair;
use crate::topic::question::plain_question_breach;
use crate::topic::question::NormalisedQuestion;
use crate::topic::question::PlainQuestionRule;
use crate::topic::question::UnicodeText;
use crate::topic::tree::Depth;

const LEVEL_NOTE_KIND: &str = "level-note";

/// A visible `findings/*.md` file: retained when it validates, in which case
/// it answers one pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    name: String,
    index: Option<u32>,
    stem: Option<Stem>,
    answer: Option<Answer>,
    depth: Depth,
}

/// The pair a retained finding answers, with its question as compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub pair: Pair,
    pub normalised: NormalisedQuestion,
}

impl Finding {
    #[must_use]
    pub fn retained(name: &str, pair: Pair, unicode: &dyn UnicodeText) -> Self {
        let normalised = NormalisedQuestion::of(&pair.question, unicode);
        Self {
            answer: Some(Answer { pair, normalised }),
            ..Self::invalid(name)
        }
    }

    #[must_use]
    pub fn invalid(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            index: Stem::index_of(name),
            stem: name.strip_suffix(".md").and_then(Stem::parse),
            answer: None,
            depth: Depth::default(),
        }
    }

    /// The finding as stamped with the depth it was researched to; an
    /// unstamped finding reads as depth 1.
    #[must_use]
    pub const fn with_depth(mut self, depth: Depth) -> Self {
        self.depth = depth;
        self
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn index(&self) -> Option<u32> {
        self.index
    }

    #[must_use]
    pub const fn stem(&self) -> Option<&Stem> {
        self.stem.as_ref()
    }

    #[must_use]
    pub const fn answer(&self) -> Option<&Answer> {
        self.answer.as_ref()
    }

    #[must_use]
    pub const fn depth(&self) -> Depth {
        self.depth
    }

    #[must_use]
    pub fn answers(
        &self,
        question: &NormalisedQuestion,
        profile: &str,
    ) -> bool {
        self.answer.as_ref().is_some_and(|answer| {
            answer.normalised == *question && answer.pair.profile == profile
        })
    }
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
    /// whether or not that note counts, so the directory can claim its pair's
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

    #[must_use]
    pub fn claim(&self, unicode: &dyn UnicodeText) -> IndexClaim {
        IndexClaim::new(self.stem.index(), self.root_question(), unicode)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::Digest;
    use super::Finding;
    use super::LevelNote;
    use super::LevelNoteFields;
    use super::NoteRejection;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::outline::Pair;
    use crate::topic::question::FakeUnicode;
    use crate::topic::question::NormalisedQuestion;
    use crate::topic::question::PlainQuestionRule;

    const PAIR: &str = "P?";

    fn lineage(text: &str) -> Lineage {
        Lineage::parse(text).unwrap_or_else(Lineage::root)
    }

    fn pair(question: &str, profile: &str) -> Pair {
        Pair {
            question: question.to_owned(),
            profile: profile.to_owned(),
        }
    }

    #[test]
    fn a_retained_finding_answers_its_pair_under_normalisation() {
        let finding =
            Finding::retained("03-a-web.md", pair(" A? ", "web"), &FakeUnicode);
        let question = NormalisedQuestion::of("A?", &FakeUnicode);
        assert!(finding.answers(&question, "web"));
        assert!(!finding.answers(&question, "arxiv"));
        assert_eq!(finding.index(), Some(3));
        assert_eq!(
            finding.stem().map(ToString::to_string).as_deref(),
            Some("03-a-web")
        );
    }

    #[test]
    fn an_invalid_finding_answers_nothing_but_keeps_its_index() {
        let finding = Finding::invalid("07-Hand Named.md");
        assert!(finding.answer().is_none());
        assert_eq!(finding.index(), Some(7));
        assert_eq!(finding.stem(), None);
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
}
