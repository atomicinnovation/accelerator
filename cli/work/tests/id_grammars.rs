//! The regular-expression grammars the ID types publish for readers that
//! match IDs in text.

use tracker::TrackerKeys;
use work::draft_id::DraftId;
use work::draft_id::ALPHABET;
use work::id_pattern::IdKey;

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
fn the_draft_grammar_accepts_every_draft_id_in_its_written_form() {
    for raw in [
        "draft-k7mq3x",
        "DRAFT-K7MQ3X",
        "draft-00000a",
        "draft-zzzzzz",
    ] {
        let draft = DraftId::parse(raw)
            .unwrap_or_else(|| unreachable!("{raw} is a draft ID"));
        assert!(
            matches_whole(DraftId::REGEX_SOURCE, draft.as_str()),
            "{raw}"
        );
    }
}

#[test]
fn the_draft_grammar_refuses_letters_outside_the_alphabet() {
    for letter in b'a'..=b'z' {
        let text = format!("draft-{}00000", char::from(letter));
        assert_eq!(
            matches_whole(DraftId::REGEX_SOURCE, &text),
            ALPHABET.contains(&letter),
            "{text}"
        );
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
