//! The `update` flow: the resolved `fields` channel, the incremental `update`
//! channel for labels/components, unassign and clear-parent, the mode-conflict
//! guard, and the `--no-notify` query.

#![cfg(feature = "test-loopback")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use http_test_support::{MockServer, RequestKey, Route};
use serde_json::Value;

const SITE: &str = r#"{"site":"acme","accountId":"acc-me"}"#;
const FIELDS: &str = r#"{"site":"acme","fields":[
    {"id":"customfield_10","key":"cf","name":"Story Points",
     "slug":"story-points","schema":{"type":"number"}}]}"#;

fn put(key: &str) -> RequestKey {
    RequestKey::put(&format!("/rest/api/3/issue/{key}"))
}

fn ok_server(key: &str) -> MockServer {
    let server = MockServer::start();
    server.route(put(key), Route::Status(204));
    server
}

fn sent(server: &MockServer, key: &str) -> Value {
    serde_json::from_slice(&server.last_body(&put(key)).expect("body"))
        .expect("json body")
}

#[test]
fn update_sets_the_named_fields_and_stamps_updated() {
    let server = ok_server("ENG-5");
    let dir = support::scratch(support::CONFIG);
    support::seed_cache(dir.path(), "site.json", SITE);
    support::seed_cache(dir.path(), "fields.json", FIELDS);

    let output = support::run(
        dir.path(),
        &server,
        &[
            "update",
            "ENG-5",
            "--summary",
            "Revised",
            "--priority",
            "Low",
            "--assignee",
            "@me",
            "--custom",
            "story-points=3",
        ],
    );
    assert!(
        output.status.success(),
        "update exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"updated\tENG-5\n", "the keyword line");
    let fields = &sent(&server, "ENG-5")["fields"];
    assert_eq!(fields["summary"], "Revised");
    assert_eq!(fields["priority"]["name"], "Low");
    assert_eq!(fields["assignee"]["accountId"], "acc-me");
    assert_eq!(fields["customfield_10"], serde_json::json!(3));
    assert!(
        fields.get("description").is_none(),
        "an unset body is never sent, so the summary is not clobbered"
    );
}

#[test]
fn incremental_labels_ride_the_update_channel() {
    let server = ok_server("ENG-5");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &[
            "update",
            "ENG-5",
            "--add-label",
            "needs-review",
            "--remove-label",
            "stale",
        ],
    );
    assert!(output.status.success());
    let body = sent(&server, "ENG-5");
    assert_eq!(
        body["update"]["labels"],
        serde_json::json!([{"add": "needs-review"}, {"remove": "stale"}])
    );
    assert!(body.get("fields").is_none(), "nothing in the set channel");
}

#[test]
fn replace_all_labels_ride_the_fields_channel() {
    let server = ok_server("ENG-5");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["update", "ENG-5", "--label", "only"],
    );
    assert!(output.status.success());
    let body = sent(&server, "ENG-5");
    assert_eq!(body["fields"]["labels"], serde_json::json!(["only"]));
    assert!(body.get("update").is_none());
}

#[test]
fn a_label_mode_conflict_is_111_before_the_wire() {
    let server = ok_server("ENG-5");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["update", "ENG-5", "--label", "a", "--add-label", "b"],
    );
    assert_eq!(output.status.code(), Some(111));
    assert_eq!(server.hits(&put("ENG-5")), 0, "nothing sent");
}

#[test]
fn an_empty_assignee_unassigns_with_a_null_account_id() {
    let server = ok_server("ENG-5");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["update", "ENG-5", "--assignee", ""],
    );
    assert!(output.status.success());
    assert_eq!(
        sent(&server, "ENG-5")["fields"]["assignee"]["accountId"],
        Value::Null
    );
}

#[test]
fn no_notify_rides_the_query_string() {
    let server = ok_server("ENG-5");
    let dir = support::scratch(support::CONFIG);

    let output = support::run(
        dir.path(),
        &server,
        &["update", "ENG-5", "--summary", "x", "--no-notify"],
    );
    assert!(output.status.success());
    assert_eq!(
        server.last_query(&put("ENG-5")).expect("a query"),
        "notifyUsers=false"
    );
}
