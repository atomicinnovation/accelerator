//! Compares both directions of this crate's ADF pipeline against the frozen
//! bash oracle corpus (`tests/support/capture-adf-oracle.sh` captured it while
//! the drivers still existed).
//!
//! The inventory and the fixtures were transcribed from the oracle by hand and
//! the Rust was written to satisfy them, so the pair agrees with itself
//! whether or not it agrees with the oracle. The frozen `oracle.out` removes
//! that doubt, and every case's committed `expected.*` cross-checks the
//! capture, so neither the corpus nor the Rust is the other's sole anchor.

#![allow(clippy::expect_used, clippy::panic)]

#[path = "support/adf_oracle.rs"]
mod adf_oracle;

use adf_oracle::{
    assemble_disagreement, case_name, cases, frozen_oracle, render_disagreement,
};
use jira_client::adf::AdfError;
use jira_client::{document_to_markdown, markdown_to_document};
use serde_json::Value;

const SEED: Option<&str> = Some("1");

fn read(case: &std::path::Path, name: &str) -> Option<String> {
    std::fs::read_to_string(case.join(name)).ok()
}

/// The class a committed abort fixture names, so the assertion is on the
/// refusal rather than on a jq message carrying an input line number.
fn expected_class(fixture: &str) -> String {
    fixture
        .lines()
        .find_map(|line| line.strip_prefix("class="))
        .expect("an abort fixture names its class")
        .to_owned()
}

const fn class_of(error: &AdfError) -> &'static str {
    match *error {
        AdfError::RootNotDoc { .. } => "root-not-doc",
        AdfError::HeadingWithoutLevel => "heading-without-level",
        AdfError::ListWithoutContent { .. } => "list-without-content",
        AdfError::UnsupportedBlockquote => "unsupported-blockquote",
        AdfError::UnsupportedTable => "unsupported-table",
        AdfError::UnsupportedNestedList => "unsupported-nested-list",
        AdfError::BadInput => "bad-input",
    }
}

#[test]
fn the_render_direction_agrees_with_the_running_jq() {
    let mut compared = 0;
    let mut disagreements = Vec::new();

    for case in cases("render-") {
        let name = case_name(&case);
        let adf = read(&case, "adf.json").expect("every case carries adf.json");
        let oracle = frozen_oracle(&case);
        let document: Value =
            serde_json::from_str(&adf).expect("the fixture is JSON");
        let rendered = document_to_markdown(&document);

        match (oracle.status, rendered) {
            (0, Ok(markdown)) => {
                if let Some(message) =
                    render_disagreement(&name, &oracle.stdout, &markdown)
                {
                    disagreements.push(message);
                }
            }
            (0, Err(error)) => disagreements.push(format!(
                "{name}: the oracle rendered but this crate refused: {error}"
            )),
            (status, Ok(markdown)) => disagreements.push(format!(
                "{name}: the oracle aborted with {status} but this crate \
                 rendered {markdown:?}"
            )),
            (status, Err(error)) => {
                let fixture = read(&case, "expected-error.txt")
                    .expect("an aborting case commits its class");
                let expected = expected_class(&fixture);
                if class_of(&error) != expected {
                    disagreements.push(format!(
                        "{name}: the oracle aborted with {status}; expected \
                         class {expected}, got {}",
                        class_of(&error)
                    ));
                }
            }
        }
        compared += 1;
    }

    assert!(
        disagreements.is_empty(),
        "the render direction disagrees with the oracle:\n{}",
        disagreements.join("\n")
    );
    assert!(
        compared > 0,
        "no case was compared — the differential proved nothing"
    );
}

#[test]
fn the_assemble_direction_agrees_with_the_running_awk_and_jq() {
    let mut compared = 0;
    let mut disagreements = Vec::new();

    for case in cases("assemble-") {
        let name = case_name(&case);
        let markdown =
            read(&case, "input.md").expect("every case carries input.md");
        let oracle = frozen_oracle(&case);
        let assembled = markdown_to_document(&markdown, SEED);

        match (oracle.status, assembled) {
            (0, Ok(document)) => {
                if let Some(message) =
                    assemble_disagreement(&name, &oracle.stdout, &document)
                {
                    disagreements.push(message);
                }
            }
            (0, Err(error)) => disagreements.push(format!(
                "{name}: the oracle assembled but this crate refused: {error}"
            )),
            (status, Ok(_)) => disagreements.push(format!(
                "{name}: the oracle rejected with {status} but this crate \
                 assembled a document"
            )),
            (status, Err(error)) => {
                if i32::from(error.code()) != status {
                    disagreements.push(format!(
                        "{name}: the oracle exited {status}, this crate \
                         reports {}",
                        error.code()
                    ));
                }
                let fixture = read(&case, "expected-error.txt")
                    .expect("a rejecting case commits its expectation");
                let expected = fixture
                    .lines()
                    .find(|line| line.contains(':'))
                    .expect("the committed message");
                if !error
                    .to_string()
                    .contains(expected.split(':').next().expect("the E_ code"))
                {
                    disagreements.push(format!(
                        "{name}: expected {expected}, got {error}"
                    ));
                }
            }
        }
        compared += 1;
    }

    assert!(
        disagreements.is_empty(),
        "the assemble direction disagrees with the oracle:\n{}",
        disagreements.join("\n")
    );
    assert!(
        compared > 0,
        "no case was compared — the differential proved nothing"
    );
}

#[test]
fn every_committed_expectation_still_matches_the_frozen_oracle() {
    // The committed expected.md / expected.adf.json files were transcribed
    // from the oracle independently of the frozen oracle.out. If a capture is
    // wrong, the two anchors disagree here and every other assertion in this
    // file is measuring the wrong thing.
    let mut checked = 0;
    for case in cases("render-") {
        let name = case_name(&case);
        let oracle = frozen_oracle(&case);
        if let Some(expected) = read(&case, "expected.md") {
            assert_eq!(
                String::from_utf8_lossy(&oracle.stdout),
                expected,
                "{name}: the committed markdown is stale"
            );
            checked += 1;
        }
    }
    for case in cases("assemble-") {
        let name = case_name(&case);
        let oracle = frozen_oracle(&case);
        if let Some(expected) = read(&case, "expected.adf.json") {
            assert_eq!(
                String::from_utf8_lossy(&oracle.stdout),
                expected,
                "{name}: the committed ADF is stale"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no committed expectation was checked");
}

#[test]
fn the_underscore_notice_is_emitted_without_failing() {
    let case = cases("assemble-")
        .into_iter()
        .find(|case| case_name(case) == "assemble-notice-underscores")
        .expect("the notice case is committed");
    let markdown = read(&case, "input.md").expect("input.md");
    let expected = read(&case, "expected-notice.txt")
        .expect("the oracle's notice is committed");

    let oracle = frozen_oracle(&case);
    let tokenised = jira_client::adf::tokenise::tokenise(&markdown)
        .expect("a notice is not a failure");

    assert_eq!(
        oracle.status, 0,
        "the notice leaves the exit status at zero"
    );
    assert!(oracle.stderr.contains(expected.trim()), "{}", oracle.stderr);
    assert_eq!(tokenised.notices, vec![expected.trim().to_owned()]);
}

#[test]
#[should_panic(expected = "the frozen oracle.out is missing")]
fn a_missing_frozen_file_reds_the_differential() {
    let dir = tempfile::tempdir().expect("tempdir");
    let _ = frozen_oracle(&dir.path().join("render-nonexistent"));
}

#[test]
#[should_panic(expected = "empty or not an integer")]
fn an_empty_status_reds_the_differential_rather_than_reading_as_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let case = dir.path().join("render-empty-status");
    std::fs::create_dir(&case).expect("case dir");
    std::fs::write(case.join("oracle.out"), b"").expect("out");
    std::fs::write(case.join("oracle.err"), b"").expect("err");
    std::fs::write(case.join("oracle-status.txt"), b"").expect("status");
    let _ = frozen_oracle(&case);
}
