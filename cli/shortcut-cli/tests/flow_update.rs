//! `update` end to end.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::RequestKey;
use serde_json::json;

const STORY: &str = "/api/v4/scratch-workspace/stories/14";

fn update(
    scenarios: &[&str],
    args: &[&str],
) -> (std::process::Output, http_test_support::MockHTTPServer) {
    let server = support::serving(scenarios);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    let output = support::run(dir.path(), &server, args);
    (output, server)
}

#[test]
fn update_sends_only_the_fields_given() {
    let (output, server) = update(
        &["story-update-200"],
        &["update", "sc-14", "--title", "New title", "--estimate", "3"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout_json(&output)["outcome"], "updated");
    assert_eq!(
        support::sent_json(&server, &RequestKey::new("PATCH", STORY)),
        json!({"name": "New title", "estimate": 3})
    );
}

#[test]
fn a_read_only_token_is_refused_naming_the_scope() {
    let (output, _) = update(
        &["story-update-403"],
        &["update", "sc-14", "--title", "New title"],
    );

    assert!(!output.status.success());
    assert!(
        support::stderr(&output).contains("the token has only the"),
        "{}",
        support::stderr(&output)
    );
}

#[test]
fn a_state_from_another_workflow_is_refused_unsent() {
    let (output, server) = update(
        &["story-show-200"],
        &["update", "sc-14", "--state", "Brief"],
    );

    assert_eq!(
        output.status.code(),
        Some(122),
        "{}",
        support::stderr(&output)
    );
    assert_eq!(server.hits(&RequestKey::new("PATCH", STORY)), 0);
}

#[test]
fn a_state_or_estimate_on_an_epic_is_a_usage_error() {
    for flag in [["--state", "To Do"], ["--estimate", "2"]] {
        let (output, _) = update(&[], &["update", "epic-17", flag[0], flag[1]]);
        assert_eq!(output.status.code(), Some(2), "{flag:?}");
        assert!(support::stderr(&output).contains("comment"));
    }
}

#[test]
fn an_unknown_owner_lists_the_members_and_sends_nothing() {
    let (output, server) =
        update(&["members-200"], &["update", "sc-14", "--owner", "nobody"]);

    assert_eq!(
        output.status.code(),
        Some(115),
        "{}",
        support::stderr(&output)
    );
    assert!(support::stderr(&output).contains("member-1"));
    assert_eq!(server.hits(&RequestKey::new("PATCH", STORY)), 0);
}

#[test]
fn naming_no_field_is_refused() {
    let (output, _) = update(&[], &["update", "sc-14"]);

    assert_eq!(output.status.code(), Some(111));
}
