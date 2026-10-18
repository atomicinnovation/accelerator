//! Compares both directions of this crate's ADF pipeline against the frozen
//! ADF oracle corpus (`tests/support/capture-adf-oracle.sh` captured it while
//! the drivers still existed).
//!
//! The inventory and the fixtures were transcribed from the oracle by hand and
//! the Rust was written to satisfy them, so the pair agrees with itself
//! whether or not it agrees with the oracle. The frozen `oracle.out` removes
//! that doubt, and every case's committed `expected.*` cross-checks the
//! capture, so neither the corpus nor the Rust is the other's sole anchor.
//!
//! The converter has since been extended past the oracle. Each case whose
//! output the extension changes is listed in [`DELIBERATE_DIVERGENCES`] with
//! its new output and the reason, and the list is exact: a listed case that
//! no longer diverges fails, as does an unlisted one that does.

#![allow(clippy::expect_used, clippy::panic)]

#[path = "support/adf_oracle.rs"]
mod adf_oracle;

use adf::{document_to_markdown, markdown_to_document, AdfError};
use adf_oracle::{
    assemble_disagreement, case_name, cases, frozen_oracle, render_disagreement,
};
use serde_json::Value;

const SEED: Option<&str> = Some("1");

struct Divergence {
    case: &'static str,
    reason: &'static str,
    now: &'static str,
}

const DELIBERATE_DIVERGENCES: &[Divergence] = &[
    Divergence {
        case: "assemble-ordered-order-always-one",
        reason: "an ordered list keeps its start number",
        now: r#"{"version":1,"type":"doc","content":[{"type":"orderedList","attrs":{"order":3},"content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"three"}]}]},{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"four"}]}]}]}]}"#,
    },
    Divergence {
        case: "assemble-reject-blockquote",
        reason: "a blockquote assembles",
        now: r#"{"version":1,"type":"doc","content":[{"type":"blockquote","content":[{"type":"paragraph","content":[{"type":"text","text":"quoted"}]}]}]}"#,
    },
    Divergence {
        case: "assemble-reject-nested-list",
        reason: "an indented bullet with no parent item is a list item",
        now: r#"{"version":1,"type":"doc","content":[{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"nested"}]}]}]}]}"#,
    },
    Divergence {
        case: "assemble-reject-nested-list-no-space",
        reason: "a marker without a following space is paragraph text",
        now: r#"{"version":1,"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"  -word continuation"}]}]}"#,
    },
    Divergence {
        case: "assemble-reject-nested-ordered",
        reason: "an indented ordered item with no parent item is a list item",
        now: r#"{"version":1,"type":"doc","content":[{"type":"orderedList","attrs":{"order":1},"content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"nested"}]}]}]}]}"#,
    },
    Divergence {
        case: "assemble-reject-table",
        reason: "a pipe row without a delimiter row is paragraph text",
        now: r#"{"version":1,"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"| a | b |"}]}]}"#,
    },
    Divergence {
        case: "render-block-placeholders",
        reason: "a blockquote and a rule render, and an empty table renders \
                 nothing",
        now: "> quoted\n\n---\n\n[unsupported ADF node: panel]\n\n\
              [unsupported ADF node: expand]\n\n\
              [unsupported ADF node: decisionList]\n\n\
              [unsupported ADF node: mediaSingle]\n\n\
              [unsupported ADF node: blockCard]",
    },
    Divergence {
        case: "render-listitem-nested-list-first-child",
        reason: "a nested list renders",
        now: "- \n  - deep",
    },
    Divergence {
        case: "render-listitem-second-child-dropped",
        reason: "a list item's later children render",
        now: "- kept\n\n  dropped\n  - also dropped",
    },
    Divergence {
        case: "render-marks-pipeline",
        reason: "adjacent runs sharing one mark set render as one run",
        now: "`code`*em***strong*****bothboth-reversed***\
              [linked](https://example.com/a)[**`all`**](https://x)",
    },
    Divergence {
        case: "render-marks-silently-ignored",
        reason: "strike renders as ~~",
        now: "~~struck~~undercolouredsubsup",
    },
    Divergence {
        case: "render-paragraph",
        reason: "text that would read as syntax is escaped",
        now: "plain text with \\* and \\_ and \\` unescaped",
    },
    Divergence {
        case: "render-text-without-text-field",
        reason: "a text node without text renders nothing",
        now: "",
    },
];

fn divergence(case: &str) -> Option<&'static Divergence> {
    DELIBERATE_DIVERGENCES
        .iter()
        .find(|divergence| divergence.case == case)
}

fn canonical_json(json: &str) -> String {
    adf_oracle::canonical(
        &serde_json::from_str(json).expect("a divergence's document is JSON"),
    )
}

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
        AdfError::Unrepresentable(_) => "unrepresentable",
        AdfError::DisallowedNesting { .. } => "disallowed-nesting",
        AdfError::BadInput => "bad-input",
    }
}

const fn expected_adf_exit(error: &AdfError) -> u16 {
    match *error {
        AdfError::RootNotDoc { .. }
        | AdfError::HeadingWithoutLevel
        | AdfError::ListWithoutContent { .. }
        | AdfError::Unrepresentable(_) => 40,
        AdfError::DisallowedNesting { .. } => 41,
        AdfError::BadInput => 42,
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

        if let Some(divergence) = divergence(&name) {
            if rendered.as_deref() != Ok(divergence.now) {
                disagreements.push(format!(
                    "{name}: diverges deliberately ({}) to {:?}, got {rendered:?}",
                    divergence.reason, divergence.now
                ));
            }
            compared += 1;
            continue;
        }

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

        if let Some(divergence) = divergence(&name) {
            let now = canonical_json(divergence.now);
            match assembled {
                Ok(document) if adf_oracle::canonical(&document) == now => {}
                other => disagreements.push(format!(
                    "{name}: diverges deliberately ({}) to {now}, got {other:?}",
                    divergence.reason
                )),
            }
            compared += 1;
            continue;
        }

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
                if i32::from(expected_adf_exit(&error)) != status {
                    disagreements.push(format!(
                        "{name}: the oracle exited {status}, this crate \
                         reports {}",
                        expected_adf_exit(&error)
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
fn every_deliberate_divergence_names_a_case_the_oracle_disagrees_with() {
    let render: Vec<String> = cases("render-")
        .iter()
        .map(|case| case_name(case))
        .collect();
    let assemble: Vec<String> = cases("assemble-")
        .iter()
        .map(|case| case_name(case))
        .collect();

    for divergence in DELIBERATE_DIVERGENCES {
        let name = divergence.case;
        let case = adf_oracle::fixtures().join(name);
        let oracle = frozen_oracle(&case);
        if render.iter().any(|case| case == name) {
            assert!(
                oracle.status != 0
                    || render_disagreement(
                        name,
                        &oracle.stdout,
                        divergence.now
                    )
                    .is_some(),
                "{name}: the oracle already renders {:?}",
                divergence.now
            );
        } else {
            assert!(
                assemble.iter().any(|case| case == name),
                "{name}: no such frozen case"
            );
            assert!(
                oracle.status != 0
                    || adf_oracle::canonical(
                        &serde_json::from_slice(&oracle.stdout)
                            .expect("the oracle emits JSON on success")
                    ) != canonical_json(divergence.now),
                "{name}: the oracle already assembles {}",
                divergence.now
            );
        }
    }
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
