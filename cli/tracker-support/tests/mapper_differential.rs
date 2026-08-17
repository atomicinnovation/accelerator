//! Drives the five bash exit-code mappers and compares every code 0-130
//! against the committed transcription.
//!
//! The fixture and any Rust written against it agree with each other whether
//! or not either agrees with the oracle. While the scripts are on disk, this
//! removes that doubt. It needs no network and no credentials, and it fails
//! rather than skips when bash is unavailable — macOS and Linux both ship it,
//! and a gate that passes when its tool is missing is exactly the failure mode
//! the contract harness was built to avoid.
//!
//! 0212 deletes this test in the same commit that deletes the scripts it
//! drives.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use support::{classify, disagreement, run_bash, table, Class, Row};

const LAST_CODE: u8 = 130;

fn statuses(script_body: &str) -> Vec<(u8, i32)> {
    let script = format!(
        "set -uo pipefail\n\
         {script_body}\n"
    );
    run_bash(&script)
        .lines()
        .map(|line| {
            let mut fields = line.split_whitespace();
            let code = fields
                .next()
                .expect("the harness prints a code")
                .parse()
                .expect("the code is numeric");
            let status = fields
                .next()
                .expect("the harness prints a status")
                .parse()
                .expect("the status is numeric");
            (code, status)
        })
        .collect()
}

fn sweep(sources: &[&str], call: &str) -> Vec<(u8, i32)> {
    let preamble = sources
        .iter()
        .map(|path| format!("source {path}"))
        .collect::<Vec<_>>()
        .join("\n");
    statuses(&format!(
        "{preamble}\n\
         for code in $(seq 0 {LAST_CODE}); do\n\
         {call}\n\
         printf '%s %s\\n' \"$code\" \"$rc\"\n\
         done"
    ))
}

const CREATE_BRIDGE: &str = "skills/work/scripts/work-item-create-remote.sh";
const UPDATE_BRIDGE: &str = "skills/work/scripts/work-item-update-remote.sh";
const LINEAR_FLOW: &str =
    "skills/integrations/linear/scripts/linear-create-flow.sh";

fn jira_create() -> Vec<(u8, i32)> {
    sweep(&[CREATE_BRIDGE], "rc=0; _wicr_map_jira \"$code\" || rc=$?")
}

fn jira_update() -> Vec<(u8, i32)> {
    sweep(&[UPDATE_BRIDGE], "rc=0; _wiur_map_jira \"$code\" || rc=$?")
}

fn linear_update() -> Vec<(u8, i32)> {
    sweep(
        &[UPDATE_BRIDGE],
        "rc=0; _wiur_map_linear \"$code\" || rc=$?",
    )
}

/// Linear's create path is two layers: the flow maps to 108/109, and the
/// bridge then maps that onto the dispatcher taxonomy.
fn linear_create() -> Vec<(u8, i32)> {
    sweep(
        &[LINEAR_FLOW, CREATE_BRIDGE],
        "inner=0; _linear_map_no_file_failure \"$code\" || inner=$?; \
         rc=0; _wicr_map_linear \"$inner\" || rc=$?",
    )
}

fn compare(
    rows: &[Row],
    provider: &str,
    operation: &str,
    observed: &[(u8, i32)],
) -> (usize, Vec<String>) {
    assert_eq!(
        observed.len(),
        usize::from(LAST_CODE) + 1,
        "{provider} {operation}: the harness must report every code"
    );
    let mut disagreements = Vec::new();
    for (code, status) in observed {
        let transcribed = classify(rows, provider, operation, *code);
        if let Some(message) =
            disagreement(provider, operation, *code, transcribed, *status)
        {
            disagreements.push(message);
        }
    }
    (observed.len(), disagreements)
}

#[test]
fn the_transcription_agrees_with_every_running_mapper() {
    let rows = table();
    let mut compared = 0;
    let mut disagreements = Vec::new();

    for (provider, operation, observed) in [
        ("jira", "create", jira_create()),
        ("jira", "update", jira_update()),
        ("linear", "create", linear_create()),
        ("linear", "update", linear_update()),
    ] {
        let (count, mut found) = compare(&rows, provider, operation, &observed);
        compared += count;
        disagreements.append(&mut found);
    }

    assert!(
        disagreements.is_empty(),
        "the transcription disagrees with the bash:\n{}",
        disagreements.join("\n")
    );
    assert!(
        compared > 0,
        "no case was compared — the differential proved nothing"
    );
    assert_eq!(compared, 4 * (usize::from(LAST_CODE) + 1));
}

#[test]
fn every_transcribed_row_names_a_recognised_provider_and_operation() {
    for row in table() {
        assert!(
            matches!(
                (row.provider.as_str(), row.operation.as_str()),
                ("jira" | "linear", "create" | "update")
            ),
            "unrecognised row: {row:?}"
        );
    }
}

#[test]
fn the_transcription_records_the_arms_of_all_five_mappers() {
    let rows = table();
    let count = |provider: &str, operation: &str| {
        rows.iter()
            .filter(|row| {
                row.provider == provider && row.operation == operation
            })
            .count()
    };
    assert_eq!(count("jira", "create"), 22);
    assert_eq!(count("jira", "update"), 21);
    assert_eq!(count("linear", "create"), 13);
    assert_eq!(count("linear", "update"), 18);
}

#[test]
fn linear_diverges_in_both_directions_between_create_and_update() {
    let rows = table();
    assert_eq!(
        classify(&rows, "linear", "create", 34),
        Class::Retryable,
        "a 200-body error is retryable on create"
    );
    assert_eq!(
        classify(&rows, "linear", "update", 34),
        Class::Terminal,
        "the same error is terminal on update: the mutation may have applied"
    );
    for code in [18, 23, 25, 27, 29] {
        assert_eq!(
            classify(&rows, "linear", "create", code),
            Class::Terminal,
            "code {code} runs the other way, with no rationale anywhere"
        );
        assert_eq!(classify(&rows, "linear", "update", code), Class::Retryable);
    }
    for code in [11, 22, 35, 36] {
        assert_eq!(classify(&rows, "linear", "create", code), Class::Retryable);
        assert_eq!(classify(&rows, "linear", "update", code), Class::Retryable);
    }
}

#[test]
fn the_jira_pair_does_not_diverge() {
    let rows = table();
    for code in [11, 12, 13, 14, 15, 17, 19, 22, 34] {
        assert_eq!(classify(&rows, "jira", "create", code), Class::Retryable);
        assert_eq!(classify(&rows, "jira", "update", code), Class::Retryable);
    }
}
