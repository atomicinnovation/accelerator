//! `comment add` end to end, for stories and epics.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::RequestKey;

#[test]
fn a_comment_on_a_story_is_added() {
    let server = support::serving(&["story-comment-201"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(
        dir.path(),
        &server,
        &["comment", "add", "sc-14", "--body", "Looks good"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout_json(&output)["outcome"], "added");
}

#[test]
fn a_comment_on_an_epic_key_reaches_the_epic() {
    let server = support::serving(&["epic-comment-201"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(
        dir.path(),
        &server,
        &["comment", "add", "epic-17", "--body", "Looks good"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(
        server.hits(&RequestKey::post(
            "/api/v4/scratch-workspace/epics/17/comments"
        )),
        1
    );
}

#[test]
fn a_comment_needs_a_body() {
    let dir = support::scratch(support::CONFIG);
    let server = support::serving(&[]);

    let output =
        support::run(dir.path(), &server, &["comment", "add", "sc-14"]);

    assert_eq!(output.status.code(), Some(91));
}
