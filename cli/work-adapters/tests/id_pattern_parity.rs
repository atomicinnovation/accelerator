//! `work::id_pattern` against the regex compiler in `corpus-adapters` it
//! replaces: the same patterns refused with the same text, the same IDs
//! rendered, the same filenames read and the same caps, apart from the
//! differences listed as expected values.

#![allow(clippy::literal_string_with_formatting_args)]

use corpus::IdScanner;
use corpus_adapters::canonicalise_id;
use corpus_adapters::compile_scan_regex;
use corpus_adapters::parse_full_id;
use corpus_adapters::pattern_max_number;
use corpus_adapters::PatternError as CompilerError;
use corpus_adapters::RegexScanner;
use work::id_pattern::IdKey;
use work::id_pattern::IdParts;
use work::id_pattern::IdPattern;
use work::id_pattern::PatternError;

type TestError = Box<dyn std::error::Error>;

const KEY: &str = "PP";

const PATTERNS: &[&str] = &[
    "{number:04d}",
    "{number}",
    "{key}-{number:04d}",
    "{project}-{number:04d}",
    "{key}-{number:06d}",
    "v{number:03d}",
    "a.{number}",
    "{tracker}",
];

const MALFORMED: &[&str] = &[
    "",
    "{number:4d}",
    "{bogus}-{number}",
    "{key}{number}",
    "{key}",
    "{number",
    "}{number}",
    "{a{b}}",
    "x/{number:04d}",
    "x\\{number:04d}",
    "x:{number:04d}",
    "x*{number:04d}",
    "x?{number:04d}",
    "x<{number:04d}",
    "x>{number:04d}",
    "x|{number:04d}",
    "x\"{number:04d}",
];

fn grammar(pattern: &str) -> Result<IdPattern, PatternError> {
    IdPattern::parse(pattern)
}

fn parsed(pattern: &str) -> Result<IdPattern, TestError> {
    Ok(IdPattern::parse(pattern)?)
}

#[test]
fn the_grammar_validates_as_the_compiler_does() {
    for pattern in PATTERNS.iter().chain(MALFORMED) {
        assert_eq!(
            grammar(pattern)
                .map(|_| ())
                .map_err(|error| error.to_string()),
            compile_scan_regex(pattern, KEY)
                .map(|_| ())
                .map_err(|error| error.to_string()),
            "{pattern:?}"
        );
    }
}

#[test]
fn a_key_is_refused_with_the_compilers_rule_five_text() {
    for value in ["1PP", "P-P", "../x", ""] {
        assert_eq!(
            IdKey::parse(value).map_err(|error| error.to_string()),
            Err(CompilerError::BadKeyValue(value.to_owned()).to_string()),
            "{value:?}"
        );
    }
    for value in ["1PP", "P-P", "../x"] {
        assert_eq!(
            compile_scan_regex("{key}-{number:04d}", value),
            Err(CompilerError::BadKeyValue(value.to_owned())),
            "{value:?}"
        );
    }
}

#[test]
fn only_the_grammar_refuses_the_patterns_it_narrows() {
    let narrowed = [
        ("x'{number:04d}", PatternError::HostileChar('\'')),
        ("x {number:04d}", PatternError::HostileChar(' ')),
        ("x${number:04d}", PatternError::HostileChar('$')),
        ("{number:020d}", PatternError::WidthOverflow(20)),
        ("{number:04d}-{number:04d}", PatternError::RepeatedToken),
        ("{key}-{key}-{number:04d}", PatternError::RepeatedToken),
        ("{key}1{number:04d}", PatternError::DigitBesideToken),
        ("v2{number:03d}", PatternError::DigitBesideToken),
        ("{{x}}{number}", PatternError::HostileChar('{')),
        ("{key}{{}}{number}", PatternError::HostileChar('{')),
    ];
    for (pattern, refusal) in narrowed {
        assert_eq!(grammar(pattern), Err(refusal), "{pattern:?}");
        assert!(compile_scan_regex(pattern, KEY).is_ok(), "{pattern:?}");
    }
    assert_eq!(
        grammar("{{/}}{number}"),
        Err(PatternError::HostileChar('{'))
    );
    assert_eq!(
        compile_scan_regex("{{/}}{number}", KEY),
        Err(CompilerError::HostileChar('/'))
    );
}

#[test]
fn the_grammar_renders_as_the_compiler_formats() -> Result<(), TestError> {
    let key = IdKey::parse(KEY)?;
    for pattern in PATTERNS.iter().filter(|pattern| **pattern != "{tracker}") {
        for number in [1u64, 42, 9999] {
            assert_eq!(
                parsed(pattern)?.render(Some(&key), number)?,
                canonicalise_id(&number.to_string(), pattern, KEY)?,
                "{pattern} {number}"
            );
        }
    }
    Ok(())
}

const FILENAMES: &[&str] = &[
    "0400-x.md",
    "00400-x.md",
    "0400.md",
    "0400x-y.md",
    "PP-0500-x.md",
    "pp-0500-x.md",
    "XY-0500-x.md",
    "v001-x.md",
    "V001-x.md",
    "PPv001-x.md",
    "a.0042-x.md",
    "A.0042-x.md",
    "PP-000042-x.md",
];

const OVERSIZED: &str = "99999999999999999999-x.md";

fn regex_reading(
    scan_regex: &str,
    filename: &str,
) -> Result<Option<(String, usize)>, TestError> {
    Ok(RegexScanner::compile(scan_regex)?
        .scan(filename)
        .map(|scan| (scan.digits, scan.match_end)))
}

fn grammar_reading(
    pattern: &IdPattern,
    filename: &str,
) -> Option<(String, usize)> {
    IdParts::parse_prefix(pattern, filename)
        .filter(|(parts, _)| {
            parts.key().is_none_or(|key| key.eq_ignore_ascii_case(KEY))
        })
        .map(|(parts, slug_start)| (parts.digits().to_owned(), slug_start))
}

#[test]
fn filenames_read_as_the_scan_regex_reads_them_but_for_letter_case(
) -> Result<(), TestError> {
    let numbered = PATTERNS
        .iter()
        .copied()
        .filter(|pattern| *pattern != "{tracker}")
        .chain(["{key}v{number:03d}"]);
    for pattern in numbered {
        let scan_regex = compile_scan_regex(pattern, KEY)?;
        let grammar = parsed(pattern)?;
        for filename in FILENAMES {
            let exact = regex_reading(&scan_regex, filename)?;
            let ignoring_case =
                regex_reading(&format!("(?i){scan_regex}"), filename)?;
            let read = grammar_reading(&grammar, filename);
            assert_eq!(read, ignoring_case, "{pattern} {filename}");
            if exact.is_some() {
                assert_eq!(read, exact, "{pattern} {filename}");
            }
        }
        assert_eq!(grammar_reading(&grammar, OVERSIZED), None, "{pattern}");
    }
    assert_eq!(
        regex_reading("^([0-9]+)-", OVERSIZED)?,
        Some(("99999999999999999999".to_owned(), 21))
    );
    Ok(())
}

#[test]
fn the_cap_matches_pattern_max_number() -> Result<(), TestError> {
    for pattern in PATTERNS.iter().chain(std::iter::once(&"{number:019d}")) {
        if let Ok(cap) = pattern_max_number(pattern) {
            assert_eq!(parsed(pattern)?.cap(), Some(cap), "{pattern}");
        } else {
            assert_eq!(parsed(pattern)?.cap(), None, "{pattern}");
        }
    }
    Ok(())
}

#[test]
fn full_ids_parse_as_parse_full_id_does() -> Result<(), TestError> {
    let keyed = [
        "{key}-{number:04d}",
        "{project}-{number:04d}",
        "{key}-{number:06d}",
        "{key}v{number:03d}",
    ];
    for pattern in keyed {
        let grammar = parsed(pattern)?;
        for id in ["PP-0042", "pp-42", "XY-7"] {
            let expected = parse_full_id(id, pattern)
                .ok()
                .map(|parsed| (parsed.key, parsed.number));
            let read = IdParts::parse(&grammar, id).map(|parts| {
                (parts.key().map(str::to_owned), parts.digits().to_owned())
            });
            assert_eq!(read, expected, "{pattern} {id}");
        }
    }
    Ok(())
}
