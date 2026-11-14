//! `transition` end to end.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::RequestKey;
use serde_json::json;

const STORY: &str = "/api/v4/scratch-workspace/stories/14";

#[test]
fn a_story_moves_to_the_named_state() {
    let server = support::serving(&["story-show-200", "story-transition-200"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output =
        support::run(dir.path(), &server, &["transition", "sc-14", "to do"]);

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout_json(&output)["outcome"], "transitioned");
    assert_eq!(
        support::sent_json(&server, &RequestKey::new("PATCH", STORY)),
        json!({"workflow_state_id": 500_000_007})
    );
}

#[test]
fn an_ambiguous_state_lists_both() {
    let server = support::serving(&["story-show-200"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_ambiguous_catalogue(dir.path());

    let output = support::run(
        dir.path(),
        &server,
        &["transition", "sc-14", "--state", "To Do"],
    );

    assert_eq!(
        output.status.code(),
        Some(123),
        "{}",
        support::stderr(&output)
    );
    let stderr = support::stderr(&output);
    assert!(
        stderr.contains("500000007") && stderr.contains("500000008"),
        "{stderr}"
    );
    assert_eq!(server.hits(&RequestKey::new("PATCH", STORY)), 0);
}

#[test]
fn a_workflow_missing_from_the_catalogue_names_it_and_init_discover() {
    let server = support::serving(&["story-other-workflow-200"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output =
        support::run(dir.path(), &server, &["transition", "sc-14", "To Do"]);

    assert_eq!(
        output.status.code(),
        Some(124),
        "{}",
        support::stderr(&output)
    );
    let stderr = support::stderr(&output);
    assert!(
        stderr.contains("777") && stderr.contains("init discover"),
        "{stderr}"
    );
}

#[test]
fn an_epic_key_is_a_usage_error() {
    let server = support::serving(&[]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output =
        support::run(dir.path(), &server, &["transition", "epic-17", "Done"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(support::stderr(&output).contains("E_NOT_A_STORY"));
}

#[test]
fn a_transition_needs_a_state() {
    let server = support::serving(&[]);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["transition", "sc-14"]);

    assert_eq!(output.status.code(), Some(121));
}
