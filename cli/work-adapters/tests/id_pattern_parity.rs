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

struct Convention {
    pattern: &'static str,
    key: Option<&'static str>,
}

const CONVENTIONS: &[Convention] = &[
    Convention {
        pattern: "{number:04d}",
        key: None,
    },
    Convention {
        pattern: "{key}-{number:04d}",
        key: Some(KEY),
    },
    Convention {
        pattern: "{tracker}",
        key: None,
    },
];

const STEMS: &[&str] = &[
    "0042-foo",
    "42-foo",
    "PP-0042-foo",
    "pp-0042-foo",
    "XY-0042-foo",
    "MY_PROJ-7-a-title",
    "0296-2026-review",
    "draft-k7mq3x-title",
    "PROJ-123-x",
    "PRO_J-123-x",
    "malformed",
];

const REFERENCES: &[&str] = &[
    "0042", "42", "PP-0042", "pp-42", "XY-7", "ENG-0042", "PROJ-123", "TBD", "",
];

fn corpus_scheme(convention: &Convention) -> corpus::WorkItemIdScheme {
    corpus::WorkItemIdScheme {
        id_pattern: convention.pattern.to_owned(),
        key: convention.key.map(str::to_owned),
    }
}

fn work_scheme(
    convention: &Convention,
) -> Result<work::id_scheme::WorkItemIdScheme, TestError> {
    Ok(work::id_scheme::WorkItemIdScheme::new(
        IdPattern::parse(convention.pattern)?,
        convention.key.map(IdKey::parse).transpose()?,
    ))
}

type Answers = Vec<(String, Option<String>)>;

fn answers(ids: &dyn corpus::IdConvention) -> Answers {
    let mut out = Vec::new();
    for stem in STEMS {
        out.push((
            format!("slug_start {stem}"),
            ids.slug_start(stem).map(|end| end.to_string()),
        ));
        out.push((
            format!("id_in_filename {stem}.md"),
            ids.id_in_filename(&format!("{stem}.md")),
        ));
    }
    for reference in REFERENCES {
        out.push((
            format!("is_id {reference}"),
            Some(ids.is_id(reference).to_string()),
        ));
        out.push((
            format!("identity_key {reference}"),
            ids.identity_key(reference),
        ));
    }
    out
}

fn changed(answers: &mut Answers, question: &str, answer: Option<&str>) {
    let slot = answers
        .iter_mut()
        .find(|(asked, _)| asked == question)
        .unwrap_or_else(|| unreachable!("{question} is asked"));
    slot.1 = answer.map(str::to_owned);
}

#[test]
fn the_scheme_is_an_id_convention() -> Result<(), TestError> {
    for convention in CONVENTIONS {
        let corpus = corpus_scheme(convention);
        let scanner = RegexScanner::compile(&compile_scan_regex(
            convention.pattern,
            convention.key.unwrap_or_default(),
        )?)?;
        let mut expected = answers(&corpus::SchemeConvention {
            scheme: &corpus,
            scanner: &scanner,
        });
        for (question, answer) in
            changes_from_the_corpus_scheme(convention.pattern)
        {
            changed(&mut expected, question, answer);
        }
        let work = work_scheme(convention)?;
        assert_eq!(answers(&work), expected, "{}", convention.pattern);
    }
    Ok(())
}

fn changes_from_the_corpus_scheme(
    pattern: &str,
) -> Vec<(&'static str, Option<&'static str>)> {
    let padded_and_folded = [
        ("identity_key pp-42", Some("PP-0042")),
        ("identity_key XY-7", Some("XY-0007")),
        ("identity_key PROJ-123", Some("PROJ-0123")),
    ];
    match pattern {
        "{number:04d}" => {
            std::iter::once(("id_in_filename 42-foo.md", Some("0042")))
                .chain(padded_and_folded)
                .collect()
        }
        "{key}-{number:04d}" => [
            ("slug_start 0042-foo", Some("5")),
            ("slug_start 42-foo", Some("3")),
            ("id_in_filename 42-foo.md", Some("PP-0042")),
            ("slug_start pp-0042-foo", Some("8")),
            ("id_in_filename pp-0042-foo.md", Some("PP-0042")),
            ("slug_start XY-0042-foo", Some("8")),
            ("id_in_filename XY-0042-foo.md", Some("XY-0042")),
            ("slug_start 0296-2026-review", Some("5")),
            ("slug_start PROJ-123-x", Some("9")),
            ("id_in_filename PROJ-123-x.md", Some("PROJ-0123")),
        ]
        .into_iter()
        .chain(padded_and_folded)
        .collect(),
        _ => vec![
            ("slug_start 0042-foo", Some("5")),
            ("id_in_filename 0042-foo.md", Some("0042")),
            ("slug_start 42-foo", Some("3")),
            ("id_in_filename 42-foo.md", Some("0042")),
            ("id_in_filename pp-0042-foo.md", Some("PP-0042")),
            ("slug_start 0296-2026-review", Some("5")),
            ("id_in_filename 0296-2026-review.md", Some("0296")),
            ("slug_start draft-k7mq3x-title", Some("13")),
            ("id_in_filename draft-k7mq3x-title.md", Some("draft-k7mq3x")),
            ("is_id 0042", Some("false")),
            ("is_id 42", Some("false")),
            ("identity_key pp-42", Some("PP-42")),
        ],
    }
}
