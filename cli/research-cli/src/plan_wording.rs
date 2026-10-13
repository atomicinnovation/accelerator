//! How a round plan's warnings, trims, skipped pairs and refused notes read
//! to whoever runs `conduct`.

use research::topic::evidence::NoteRejection;
use research::topic::plan::Skip;
use research::topic::plan::SkipReason;
use research::topic::plan::Warning;
use research::topic::question::PlainQuestionRule;
use research::topic::question::LONGEST_PLAIN_QUESTION;
use research::topic::tree::Trim;

pub fn warning(warning: &Warning) -> String {
    match warning {
        Warning::UnseparatedProfiles { line } => format!(
            "outline line {line} names 'profiles:' without a '—', '–', or \
             '--' separator before it, so it is researched on the web"
        ),
        Warning::UnmatchedFinding { name, question } => format!(
            "finding {name} answers '{question}', which no outline item asks"
        ),
        Warning::IndexesExhausted { question } => format!(
            "no finding index is left to allocate to '{question}', so it is \
             not planned"
        ),
    }
}

pub fn trim(trim: &Trim) -> String {
    format!(
        "level note {}.levels/{} records {} follow-ups, over its cap of {}; \
         trimmed {}",
        trim.at.stem,
        trim.at.lineage,
        trim.recorded,
        trim.cap,
        trim.trimmed()
    )
}

pub fn skip_reason(skip: &Skip) -> String {
    let profile = &skip.pair.profile;
    match skip.reason {
        SkipReason::NotInBrief => {
            format!("'{profile}' is not in the brief's source_profiles")
        }
        SkipReason::NotInstalled => {
            format!("no '{profile}-profile' skill is installed")
        }
    }
}

pub fn rejection(rejection: &NoteRejection) -> String {
    match rejection {
        NoteRejection::FailsValidation { code, field: None } => {
            format!("fails validation: {code}")
        }
        NoteRejection::FailsValidation {
            code,
            field: Some(field),
        } => format!("fails validation: {code} on {field}"),
        NoteRejection::WrongKind => "kind is not level-note".to_owned(),
        NoteRejection::LevelDisagreesWithLineage => {
            "level does not match its lineage".to_owned()
        }
        NoteRejection::MissingQuestion => "question is missing".to_owned(),
        NoteRejection::MalformedFollowUps => {
            "follow_ups is not a list of questions".to_owned()
        }
        NoteRejection::NotAPlainQuestion { follow_up, rule } => {
            let breach = match rule {
                PlainQuestionRule::TooLong => format!(
                    "is longer than {LONGEST_PLAIN_QUESTION} characters"
                ),
                PlainQuestionRule::HiddenCharacter => {
                    "contains a hidden or control character".to_owned()
                }
                PlainQuestionRule::Link => {
                    "contains a link or host name".to_owned()
                }
            };
            format!("follow-up {follow_up} {breach}")
        }
        NoteRejection::QuestionDisagreesWithCandidate => {
            "answers a different question from its node".to_owned()
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::num::NonZeroUsize;

    use research::topic::evidence::NoteRejection;
    use research::topic::layout::lineage::Lineage;
    use research::topic::layout::note_ref::NoteRef;
    use research::topic::layout::stem::Stem;
    use research::topic::outline::Pair;
    use research::topic::plan::Skip;
    use research::topic::plan::SkipReason;
    use research::topic::plan::Warning;
    use research::topic::question::PlainQuestionRule;
    use research::topic::tree::Trim;

    use super::rejection;
    use super::skip_reason;
    use super::trim;
    use super::warning;

    #[test]
    fn every_warning_reads_with_what_it_concerns() {
        let rows = [
            (
                Warning::UnseparatedProfiles { line: 2 },
                "outline line 2 names 'profiles:' without a '—', '–', or \
                 '--' separator before it, so it is researched on the web",
            ),
            (
                Warning::UnmatchedFinding {
                    name: "01-x.md".into(),
                    question: "Why?".into(),
                },
                "finding 01-x.md answers 'Why?', which no outline item asks",
            ),
            (
                Warning::IndexesExhausted {
                    question: "M?".into(),
                },
                "no finding index is left to allocate to 'M?', so it is not \
                 planned",
            ),
        ];
        for (given, text) in rows {
            assert_eq!(warning(&given), text);
        }
    }

    #[test]
    fn a_trim_names_its_note_and_how_much_it_dropped() {
        let given = Trim {
            at: NoteRef {
                stem: Stem::parse("01-a-web").expect("a stem"),
                lineage: Lineage::root(),
            },
            recorded: 5,
            cap: 4,
        };
        assert_eq!(
            trim(&given),
            "level note 01-a-web.levels/1 records 5 follow-ups, over its cap \
             of 4; trimmed 1"
        );
    }

    #[test]
    fn a_skip_explains_why_its_profile_cannot_be_researched() {
        let skip = |profile: &str, reason| Skip {
            pair: Pair {
                question: "A?".into(),
                profile: profile.into(),
            },
            reason,
        };
        assert_eq!(
            skip_reason(&skip("arxiv", SkipReason::NotInBrief)),
            "'arxiv' is not in the brief's source_profiles"
        );
        assert_eq!(
            skip_reason(&skip("crossref", SkipReason::NotInstalled)),
            "no 'crossref-profile' skill is installed"
        );
    }

    #[test]
    fn every_note_rejection_reads_as_its_reason() {
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
        for (given, text) in rows {
            assert_eq!(rejection(&given), text);
        }
    }
}
