//! Proves the ADF differential can fail.
//!
//! A comparison nobody has watched reject anything is indistinguishable from
//! one that returns `None` unconditionally, so the wrong rendering is planted
//! here in a committed test rather than by hand in a session nobody can re-run.

#![allow(clippy::expect_used)]

#[path = "support/adf_oracle.rs"]
mod adf_oracle;

use adf_oracle::{assemble_disagreement, render_disagreement};
use serde_json::json;

#[test]
fn a_wrong_rendering_is_reported_and_names_the_case() {
    let message =
        render_disagreement("render-marks-pipeline", b"`code`\n", "code")
            .expect(
                "a rendering that disagrees with the oracle must be reported",
            );

    assert!(message.contains("render-marks-pipeline"), "{message}");
    assert!(message.contains("oracle:"), "{message}");
    assert!(message.contains("rust:"), "{message}");
}

#[test]
fn a_missing_trailing_newline_is_not_reported_as_a_disagreement() {
    // The oracle's jq -r adds exactly one; the renderer deliberately returns
    // the joined blocks without it.
    assert!(
        render_disagreement("case", b"one\n\ntwo\n", "one\n\ntwo").is_none()
    );
}

#[test]
fn an_empty_document_must_render_to_zero_bytes_not_a_newline() {
    assert!(render_disagreement("case", b"", "").is_none());
    assert!(
        render_disagreement("case", b"\n", "").is_some(),
        "a stray newline is a disagreement, not a rounding error"
    );
}

#[test]
fn a_wrong_assembly_is_reported_with_both_documents() {
    let oracle = br#"{"version":1,"type":"doc","content":[]}"#;
    let message = assemble_disagreement(
        "assemble-empty-input",
        oracle,
        &json!({"version": 1, "type": "doc", "content": [{"type": "paragraph"}]}),
    )
    .expect("an assembly that disagrees with the oracle must be reported");

    assert!(message.contains("assemble-empty-input"), "{message}");
    assert!(message.contains("paragraph"), "{message}");
}

#[test]
fn key_order_alone_is_not_a_disagreement() {
    // jq emits version, type, content; serde_json emits them sorted. The
    // comparison canonicalises both, because enabling preserve_order is
    // forbidden workspace-wide.
    let oracle = br#"{"version":1,"type":"doc","content":[]}"#;
    assert!(assemble_disagreement(
        "case",
        oracle,
        &json!({"content": [], "type": "doc", "version": 1})
    )
    .is_none());
}
