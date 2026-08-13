//! The cue-phrase pattern slice against the file that declares itself
//! canonical for both `extract-work-items` and this audit.
//!
//! The patterns are compiled in so the domain crate needs no regex engine, so
//! the shared-file contract is asserted here rather than assumed.

use design::cue_phrase_audit::CASE_SENSITIVE_CUE_PHRASE_PATTERN;
use design::CUE_PHRASE_PATTERNS;

const CANONICAL: &str =
    include_str!("../../../scripts/extract-work-items-cue-phrases.txt");

/// One ERE alternative per non-comment, non-blank line, as the file's own
/// header states.
fn alternatives() -> Vec<&'static str> {
    CANONICAL
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

#[test]
fn the_compiled_patterns_are_exactly_the_canonical_alternatives() {
    let mut expected = CUE_PHRASE_PATTERNS.to_vec();
    expected.push(CASE_SENSITIVE_CUE_PHRASE_PATTERN);
    assert_eq!(
        alternatives(),
        expected,
        "the compiled slice drifted from \
         scripts/extract-work-items-cue-phrases.txt"
    );
}

/// The case-sensitive alternative is the last line, and it is the only one
/// carrying an uppercase-letter requirement — the property that distinguishes
/// `implement Foo` from `implement foo`.
#[test]
fn only_the_case_sensitive_alternative_pins_letter_case() {
    assert_eq!(
        alternatives().last().copied(),
        Some(CASE_SENSITIVE_CUE_PHRASE_PATTERN)
    );
    assert!(CUE_PHRASE_PATTERNS
        .iter()
        .all(|pattern| !pattern.contains("[A-Z]")));
}
