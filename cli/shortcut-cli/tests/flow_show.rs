//! `show` end to end, for keys and for web-app URLs.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::RequestKey;

const STORY: &str = "/api/v4/scratch-workspace/stories/14";

fn show(scenarios: &[&str], args: &[&str]) -> std::process::Output {
    let server = support::serving(scenarios);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());
    support::run(dir.path(), &server, args)
}

#[test]
fn a_story_key_shows_the_story_with_its_key() {
    let output = show(&["story-show-200"], &["show", "sc-14"]);

    assert!(output.status.success(), "{}", support::stderr(&output));
    let stdout = support::stdout_json(&output);
    assert_eq!(stdout["outcome"], "found");
    assert_eq!(stdout["key"], "sc-14");
    assert_eq!(stdout["entity"]["name"], "Contract story");
    assert!(stdout["entity"].get("comments").is_none());
}

#[test]
fn a_story_url_echoes_the_story_key() {
    let output = show(
        &["story-show-200"],
        &[
            "show",
            "https://app.shortcut.com/scratch-workspace/story/14/contract-story",
        ],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout_json(&output)["key"], "sc-14");
}

#[test]
fn an_epic_url_echoes_the_epic_key() {
    let output = show(
        &["epic-show-200"],
        &["show", "https://app.shortcut.com/scratch-workspace/epic/17"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout_json(&output)["key"], "epic-17");
}

#[test]
fn a_url_for_another_workspace_is_a_usage_error_that_sends_nothing() {
    let server = support::serving(&["story-show-200"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(
        dir.path(),
        &server,
        &["show", "https://app.shortcut.com/elsewhere/story/14"],
    );

    assert_eq!(output.status.code(), Some(81));
    assert!(support::stderr(&output).contains("elsewhere"));
    assert_eq!(server.hits(&RequestKey::get(STORY)), 0);
}

#[test]
fn text_that_is_not_a_reference_is_refused() {
    let output = show(&[], &["show", "ENG-14"]);

    assert_eq!(output.status.code(), Some(80));
}

#[test]
fn a_missing_story_is_not_found() {
    let output = show(&["story-show-404"], &["show", "sc-99"]);

    assert_eq!(output.status.code(), Some(82));
    assert_eq!(support::stdout_json(&output)["outcome"], "not-found");
}

#[test]
fn comments_brings_the_most_recent_comments() {
    let output = show(
        &["story-show-200", "story-comments-200"],
        &["show", "sc-14", "--comments", "2"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    let texts: Vec<String> = support::stdout_json(&output)["comments"]
        .as_array()
        .expect("comments")
        .iter()
        .filter_map(|comment| comment["text"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(texts, ["Second", "Third"]);
}

#[test]
fn without_a_catalogue_show_says_how_to_get_one() {
    let server = support::serving(&["story-show-200"]);
    let dir = support::scratch(support::CONFIG);

    let output = support::run(dir.path(), &server, &["show", "sc-14"]);

    assert_eq!(output.status.code(), Some(30));
    assert!(support::stderr(&output).contains("init-shortcut"));
}
