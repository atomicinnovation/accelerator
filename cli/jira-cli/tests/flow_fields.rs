//! The `fields` cache reads through the marker-checked cache path (Decision 21):
//! a markerless bash-era cache reads, and a cache carrying an unrecognised
//! version marker fails closed rather than feeding stale values onward. These
//! reads are offline — the client builds from config without a network call.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use support::Token;

fn seed(dir: &Path, file: &str, contents: &str) {
    let cache = dir.join(".accelerator/state/integrations/jira");
    std::fs::create_dir_all(&cache).expect("mkdir cache");
    std::fs::write(cache.join(file), contents).expect("seed cache");
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    support::run_with(dir, args, None, &Token::Present)
}

#[test]
fn a_markerless_fields_cache_resolves() {
    let dir = support::scratch(support::CONFIG);
    seed(
        dir.path(),
        "fields.json",
        r#"{"fields":[{"id":"cf-10","name":"Sprint","slug":"sprint"}]}"#,
    );

    let output = run(dir.path(), &["fields", "resolve", "Sprint"]);
    assert!(
        output.status.success(),
        "exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"cf-10\n", "the field id is emitted");
}

#[test]
fn an_incompatible_marker_fails_closed_with_the_corrupt_code() {
    let dir = support::scratch(support::CONFIG);
    seed(dir.path(), "fields.json", r#"{"fields":[]}"#);
    seed(dir.path(), ".cache-version.json", r#"{"version":999}"#);

    let output = run(dir.path(), &["fields", "list"]);
    assert_eq!(
        output.status.code(),
        Some(52),
        "an unrecognised cache version → FIELD_CACHE_CORRUPT"
    );
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("E_CACHE_INCOMPATIBLE"));
}

#[test]
fn a_missing_cache_reports_the_missing_code() {
    let dir = support::scratch(support::CONFIG);
    let output = run(dir.path(), &["fields", "list"]);
    assert_eq!(
        output.status.code(),
        Some(51),
        "no cache → FIELD_CACHE_MISSING"
    );
}
