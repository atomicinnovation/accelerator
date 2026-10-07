#![allow(clippy::literal_string_with_formatting_args)]

use corpus::work_item_id::CanonicaliseError;
use corpus::work_item_id::WorkItemIdCanonicaliser;
use corpus_adapters::canonicalise_id;
use corpus_adapters::PatternCanonicaliser;

#[test]
fn canonicalises_through_the_configured_pattern() {
    assert_eq!(
        PatternCanonicaliser.canonicalise("42", "{key}-{number:04d}", "PROJ"),
        Ok("PROJ-0042".to_owned())
    );
}

#[test]
fn each_input_refusal_keeps_its_own_arm_and_text() {
    let cases = [
        (("", "{number:04d}", ""), CanonicaliseError::EmptyInput),
        (
            ("42", "{key}-{number:04d}", ""),
            CanonicaliseError::MissingKey,
        ),
        (
            ("not-an-id", "{number:04d}", ""),
            CanonicaliseError::UnrecognisedIdShape("not-an-id".to_owned()),
        ),
        (
            ("99999999999999999999", "{number:04d}", ""),
            CanonicaliseError::NoMatch,
        ),
    ];
    for ((input, pattern, key), expected) in cases {
        let refusal = PatternCanonicaliser.canonicalise(input, pattern, key);
        assert_eq!(refusal, Err(expected), "{input:?} {pattern:?}");
        assert_eq!(
            refusal.map_err(|error| error.to_string()),
            canonicalise_id(input, pattern, key)
                .map_err(|error| error.to_string()),
        );
    }
}

#[test]
fn each_malformed_pattern_refusal_carries_the_pattern_error_text() {
    let cases = [
        ("", "", "pattern is empty"),
        ("}{number}", "", "unmatched '}' at offset 0"),
        (
            "{a{b}{number}",
            "",
            "nested '{' in token starting at offset 0",
        ),
        ("{number", "", "unclosed token starting at offset 0"),
        (
            "{key}{number}",
            "PROJ",
            "dynamic tokens must be separated by literal text (rule 3)",
        ),
        (
            "{key}-{number}",
            "1PROJ",
            "key value '1PROJ' must match [A-Za-z][A-Za-z0-9]* (rule 5)",
        ),
        (
            "{number:9x}",
            "",
            "{number} format spec '9x' must match 0Nd (rule 4)",
        ),
        ("{bogus}-{number}", "", "unknown token '{bogus}' in pattern"),
        (
            "a/{number}",
            "",
            "literal '/' is forbidden in patterns (rule 2)",
        ),
        (
            "abc",
            "",
            "pattern must contain at least one {number} token (rule 1)",
        ),
    ];
    for (pattern, key, text) in cases {
        assert_eq!(
            PatternCanonicaliser.canonicalise("42", pattern, key),
            Err(CanonicaliseError::MalformedPattern(text.to_owned())),
            "{pattern:?}"
        );
    }
}
