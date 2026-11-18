//! The regular-expression grammars the ID types publish for readers that
//! match IDs in text.

// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#![allow(clippy::literal_string_with_formatting_args)]

use tracker::TrackerKeys;
use work::draft_id::DraftId;
use work::draft_id::ALPHABET;
use work::id_pattern::IdKey;
use work::id_pattern::IdPattern;
use work::id_scheme::IdScheme;

fn matches_whole(source: &str, text: &str) -> bool {
    regex::Regex::new(&format!("^(?:{source})$"))
        .unwrap_or_else(|error| unreachable!("{error}"))
        .is_match(text)
}

#[test]
fn the_key_grammar_accepts_exactly_the_keys_that_parse() {
    for text in ["PP", "eng", "p1", "A", "1PP", "P_P", "P-P", "", "PP ", "Ä"] {
        assert_eq!(
            matches_whole(IdKey::REGEX_SOURCE, text),
            IdKey::parse(text).is_ok(),
            "{text}"
        );
    }
}

#[test]
fn the_draft_grammar_accepts_every_draft_id_in_any_case() {
    for raw in [
        "draft-k7mq3x",
        "DRAFT-K7MQ3X",
        "draft-00000a",
        "draft-zzzzzz",
    ] {
        let draft = DraftId::parse(raw)
            .unwrap_or_else(|| unreachable!("{raw} is a draft ID"));
        assert!(matches_whole(DraftId::REGEX_SOURCE, raw), "{raw}");
        assert!(
            matches_whole(DraftId::REGEX_SOURCE, draft.as_str()),
            "{raw}"
        );
    }
}

#[test]
fn the_draft_grammar_refuses_letters_outside_the_alphabet() {
    for letter in b'a'..=b'z' {
        for text in [
            format!("draft-{}00000", char::from(letter)),
            format!("DRAFT-{}00000", char::from(letter).to_ascii_uppercase()),
        ] {
            assert_eq!(
                matches_whole(DraftId::REGEX_SOURCE, &text),
                ALPHABET.contains(&letter),
                "{text}"
            );
        }
    }
    assert!(!matches_whole(DraftId::REGEX_SOURCE, "draft-k7mq3"));
    assert!(!matches_whole(DraftId::REGEX_SOURCE, "draft-k7mq3xy"));
}

#[test]
fn each_trackers_key_grammar_accepts_exactly_its_keys() {
    let samples = [
        "PP-760",
        "eng-42",
        "MY_PROJ-7",
        "PRO_J-1",
        "p1-7",
        "0230",
        "PP-",
        "-760",
        "1PP-760",
        "PP-76a",
        "draft-k7mq3x",
        "PROJ",
        "",
        "PP-7-8",
    ];
    for keys in [
        TrackerKeys::Jira,
        TrackerKeys::Linear,
        TrackerKeys::AnySupported,
    ] {
        for token in samples {
            assert_eq!(
                matches_whole(keys.regex_source(), token),
                keys.is_key(token),
                "{keys:?} {token}"
            );
        }
    }
}

fn scheme(pattern: &str, key: Option<&str>) -> IdScheme {
    IdScheme::under_any_tracker(
        IdPattern::parse(pattern)
            .unwrap_or_else(|error| unreachable!("{error}")),
        key.and_then(|key| IdKey::parse(key).ok()),
    )
}

#[test]
fn a_scheme_grammar_accepts_every_id_its_pattern_renders_and_a_bare_number() {
    for (pattern, key) in [
        ("{key}_{number:04d}", Some("PP")),
        ("{key}-{number:04d}", Some("PP")),
        ("v{number:03d}", None),
        ("a.{number}", None),
        ("{number:04d}", None),
    ] {
        let scheme = scheme(pattern, key);
        let source = scheme.regex_source();
        let cap = scheme.pattern().cap().unwrap_or(99_999).min(99_999);
        for number in [1, 42, cap] {
            let id = scheme
                .pattern()
                .render(scheme.key(), number)
                .unwrap_or_else(|error| unreachable!("{error}"));
            assert!(matches_whole(&source, &id), "{pattern}: {id}");
        }
        assert!(matches_whole(&source, "42"), "{pattern}: a bare number");
    }
}

#[test]
fn a_scheme_grammar_reads_an_id_in_any_case_as_the_scheme_does() {
    assert!(matches_whole(
        &scheme("v{number:03d}", None).regex_source(),
        "V001"
    ));
    assert!(matches_whole(
        &scheme("{tracker}", None).regex_source(),
        "DRAFT-K7MQ3X"
    ));
}

#[test]
fn a_scheme_grammar_reads_a_literal_metacharacter_as_itself() {
    let source = scheme("a.{number}", None).regex_source();
    assert!(matches_whole(&source, "a.7"));
    assert!(!matches_whole(&source, "aX7"));
}

#[test]
fn a_tracker_scheme_grammar_reads_keys_by_its_trackers_rules_and_drafts() {
    let under = |keys| {
        IdScheme::under_tracker(
            IdPattern::parse("{tracker}")
                .unwrap_or_else(|error| unreachable!("{error}")),
            None,
            keys,
        )
        .regex_source()
    };
    let jira = under(TrackerKeys::Jira);
    let linear = under(TrackerKeys::Linear);
    for source in [&jira, &linear] {
        assert!(matches_whole(source, "ENG-7"));
        assert!(matches_whole(source, "draft-k7mq3x"));
        assert!(!matches_whole(source, "0017"));
    }
    assert!(matches_whole(&jira, "MY_PROJ-7"));
    assert!(!matches_whole(&linear, "MY_PROJ-7"));
}
