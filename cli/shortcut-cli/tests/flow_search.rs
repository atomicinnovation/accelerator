//! `search` end to end: the flags lower to one v4 query, scoped to the base
//! team.
#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::RequestKey;

const SEARCH: &str = "/api/v4/scratch-workspace/stories/search";

#[test]
fn search_lists_the_matching_stories_by_key() {
    let server = support::serving(&["search-team-200"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(
        dir.path(),
        &server,
        &["search", "--type", "story", "--state", "Backlog"],
    );

    assert!(output.status.success(), "{}", support::stderr(&output));
    let stdout = support::stdout_json(&output);
    assert_eq!(stdout["outcome"], "results");
    assert_eq!(stdout["stories"][0]["key"], "sc-14");
    assert_eq!(stdout["stories"][0]["state"], "Backlog");
    assert_eq!(stdout["truncated"], false);
    let query = server
        .last_query(&RequestKey::get(SEARCH))
        .expect("a search was sent");
    assert!(query.contains("type%3Afeature"), "{query}");
    assert!(query.contains("Backlog"), "{query}");
}

#[test]
fn no_match_is_empty() {
    let server = support::serving(&["search-team-empty-200"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(dir.path(), &server, &["search", "--text", "x"]);

    assert!(output.status.success(), "{}", support::stderr(&output));
    assert_eq!(support::stdout_json(&output)["outcome"], "empty");
}

#[test]
fn limit_caps_the_stories_printed() {
    let server = support::serving(&["search-team-200"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(dir.path(), &server, &["search", "--limit", "0"]);

    let stdout = support::stdout_json(&output);
    assert_eq!(stdout["stories"].as_array().map(Vec::len), Some(0));
    assert_eq!(stdout["truncated"], true);
    assert_eq!(stdout["outcome"], "truncated");
}

#[test]
fn an_unusable_filter_is_refused_before_sending() {
    let server = support::serving(&["search-team-200"]);
    let dir = support::scratch(support::CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(
        dir.path(),
        &server,
        &["search", "--owner", "ada lovelace"],
    );

    assert_eq!(output.status.code(), Some(75));
    assert_eq!(server.hits(&RequestKey::get(SEARCH)), 0);
}

#[test]
fn without_a_team_key_search_names_the_fix() {
    let server = support::serving(&["search-team-200"]);
    let dir = support::scratch(support::KEYLESS_CONFIG);
    support::seed_catalogue(dir.path());

    let output = support::run(dir.path(), &server, &["search"]);

    assert_eq!(output.status.code(), Some(31));
    assert!(support::stderr(&output).contains("shortcut.team_key"));
}
