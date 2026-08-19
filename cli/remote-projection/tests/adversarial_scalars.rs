//! Holds the raw-token reader against `jq -cS` for every scalar shape a real
//! tenant could put in a numeric custom field.
//!
//! The offline corpus fixtures contain no numbers at all, so without this table
//! the parity assertions pass vacuously — and a formatting difference on one
//! numeric field reclassifies every item carrying it as remotely modified on
//! the first live sync.

#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;

use remote_projection::json::{parse, Limits, Node};

struct Row {
    literal: String,
    jq: String,
    verdict: String,
    note: String,
}

fn table() -> Vec<Row> {
    let raw = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/adversarial-scalars.txt"),
    )
    .expect("the committed table is readable");
    raw.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 4, "four columns: {line:?}");
            Row {
                literal: fields[0].to_owned(),
                jq: fields[1].to_owned(),
                verdict: fields[2].to_owned(),
                note: fields[3].to_owned(),
            }
        })
        .collect()
}

fn canonical(literal: &str) -> String {
    parse(literal, &Limits::default())
        .expect("every row parses")
        .canonical()
}

#[test]
fn the_table_records_the_jq_version_it_was_generated_against() {
    let raw = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/adversarial-scalars.txt"),
    )
    .expect("the committed table is readable");

    assert!(
        raw.lines()
            .any(|line| line.starts_with("# jq version: jq-")),
        "the table must name the jq it was taken against"
    );
}

#[test]
fn every_matching_row_renders_exactly_as_jq_does() {
    let rows = table();
    let mut asserted = 0;

    for row in rows.iter().filter(|row| row.verdict == "match") {
        assert_eq!(
            canonical(&row.literal),
            row.jq,
            "{}: {}",
            row.literal,
            row.note
        );
        asserted += 1;
    }

    assert!(asserted > 0, "the table must drive the assertions");
    assert_eq!(
        asserted
            + rows
                .iter()
                .filter(|row| row.verdict == "divergence")
                .count(),
        rows.len(),
        "every row is either a match or a listed divergence — the coverage \
         guard: a row added without a verdict fails here"
    );
}

#[test]
fn every_divergent_row_diverges_for_the_recorded_reason() {
    for row in table().iter().filter(|row| row.verdict == "divergence") {
        assert_ne!(
            canonical(&row.literal),
            row.jq,
            "{} is listed as a divergence but now matches — reclassify it",
            row.literal
        );
        assert!(
            row.literal.contains('e')
                || row.literal.contains('E')
                || row.literal.contains("\\u007f"),
            "the only accepted divergences are exponent notation, which jq \
             re-renders through its decimal library, and U+007F, which jq \
             escapes and serde_json emits raw: {}",
            row.literal
        );
    }
}

#[test]
fn a_number_keeps_the_bytes_the_tracker_sent() {
    // What plain serde_json would lose: the trailing zeros and the precision.
    assert_eq!(canonical("1.500"), "1.500");
    assert_eq!(canonical("9007199254740993"), "9007199254740993");
    assert_eq!(
        serde_json::to_string(
            &serde_json::from_str::<serde_json::Value>("1.500")
                .expect("serde_json parses it")
        )
        .expect("it re-renders"),
        "1.5",
        "the reader exists because this is what a Value round trip does"
    );
}

#[test]
fn keys_are_sorted_and_output_is_compact() {
    assert_eq!(
        canonical("{\"b\": 1, \"a\": [2, {\"d\": 3, \"c\": 4}]}"),
        "{\"a\":[2,{\"c\":4,\"d\":3}],\"b\":1}"
    );
}

#[test]
fn a_document_deeper_than_the_bound_is_a_typed_error() {
    let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
    let error = parse(&deep, &Limits::default())
        .expect_err("nesting beyond the bound is refused");

    assert!(
        matches!(error, remote_projection::json::JsonError::Depth { .. }),
        "{error}"
    );
}

#[test]
fn an_unbounded_numeric_literal_is_a_typed_error() {
    let long = "9".repeat(1024);
    let error = parse(&long, &Limits::default())
        .expect_err("an unbounded numeric literal is refused");

    assert!(
        matches!(
            error,
            remote_projection::json::JsonError::NumberTooLong { .. }
        ),
        "{error}"
    );
}

#[test]
fn a_truncated_document_is_a_typed_error_not_a_panic() {
    for truncated in ["{\"a\":", "{\"a\":1", "[1,", "\"unterminated", "{,}"] {
        assert!(
            parse(truncated, &Limits::default()).is_err(),
            "{truncated:?} must be refused"
        );
    }
}

#[test]
fn trailing_input_after_a_document_is_refused() {
    assert!(matches!(
        parse("{} {}", &Limits::default()),
        Err(remote_projection::json::JsonError::Trailing { .. })
    ));
}

#[test]
fn the_raw_projection_matches_the_value_projection_where_no_number_is_present()
{
    let payload = r#"{"fields":{"summary":"S","description":{"type":"doc"}}}"#;
    let raw = remote_projection::project_raw(
        remote_projection::Integration::Jira,
        remote_projection::Op::Body,
        payload,
    )
    .expect("the payload parses");
    let parsed: serde_json::Value =
        serde_json::from_str(payload).expect("the payload parses");
    let via_value = remote_projection::project(
        remote_projection::Integration::Jira,
        remote_projection::Op::Body,
        &parsed,
    );

    assert_eq!(raw, via_value);
}

#[test]
fn an_absent_description_projects_as_the_literal_null() {
    let raw = remote_projection::project_raw(
        remote_projection::Integration::Jira,
        remote_projection::Op::Body,
        r#"{"fields":{"summary":"S"}}"#,
    )
    .expect("the payload parses");

    assert_eq!(raw, "S\nnull");
    assert_eq!(Node::Null.canonical(), "null");
}
