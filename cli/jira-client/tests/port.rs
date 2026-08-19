//! The four port operations, against a mock server.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::JiraClient;
use serde_json::Value;
use support::client::{brief, client_for, PROJECT};
use tracker::{ExternalId, RemoteTimestamp, RemoteTracker as _, TrackerError};

const ISSUE: &str = "/rest/api/3/issue";
const SEARCH: &str = "/rest/api/3/search/jql";

fn id(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

fn issue_payload(key: &str, updated: Option<&str>) -> String {
    let stamp = updated
        .map_or_else(|| "null".to_owned(), |value| format!("\"{value}\""));
    format!(
        "{{\"key\":\"{key}\",\"fields\":{{\"updated\":{stamp},\
         \"summary\":\"A summary\",\
         \"description\":{{\"type\":\"doc\",\"version\":1}}}}}}"
    )
}

fn search_page(keys: &[&str], cursor: Option<&str>) -> String {
    let issues: Vec<String> = keys
        .iter()
        .map(|key| {
            format!(
                "{{\"key\":\"{key}\",\
                 \"fields\":{{\"updated\":\"2026-01-01T00:00:00.000+0000\"}}}}"
            )
        })
        .collect();
    let tail = cursor.map_or_else(String::new, |token| {
        format!(",\"nextPageToken\":\"{token}\"")
    });
    format!("{{\"issues\":[{}]{tail}}}", issues.join(","))
}

#[test]
fn create_posts_the_issue_and_returns_its_key() {
    let server = MockServer::start();
    let key = RequestKey::post(ISSUE);
    server.route(
        key.clone(),
        Route::Json {
            status: 201,
            body: "{\"id\":\"1\",\"key\":\"ENG-42\",\"self\":\"https://x\"}"
                .to_owned(),
        },
    );
    let client = client_for(&server, brief());

    let created = client
        .create("A title", "A body\n", "Bug")
        .expect("create succeeds");

    assert_eq!(created, id("ENG-42"));
    let sent: Value = serde_json::from_slice(
        &server.last_body(&key).expect("the request carried a body"),
    )
    .expect("the body is JSON");
    assert_eq!(sent["fields"]["project"]["key"], PROJECT);
    assert_eq!(sent["fields"]["summary"], "A title");
    assert_eq!(sent["fields"]["issuetype"]["name"], "Bug");
    assert_eq!(sent["fields"]["description"]["type"], "doc");
}

#[test]
fn create_with_no_kind_uses_the_default_issue_type() {
    let server = MockServer::start();
    let key = RequestKey::post(ISSUE);
    server.route(
        key.clone(),
        Route::Json {
            status: 201,
            body: "{\"key\":\"ENG-1\"}".to_owned(),
        },
    );
    let client = client_for(&server, brief());

    client
        .create("A title", "A body\n", "")
        .expect("create succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert_eq!(sent["fields"]["issuetype"]["name"], "Task");
}

#[test]
fn a_create_whose_key_cannot_be_written_back_is_terminal() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(ISSUE),
        Route::Json {
            status: 201,
            body: "{\"key\":\"---ENG-1\"}".to_owned(),
        },
    );
    let client = client_for(&server, brief());

    let error = client
        .create("A title", "A body\n", "Bug")
        .expect_err("an unusable key is a failure");

    assert!(
        matches!(error, TrackerError::Terminal { .. }),
        "the issue exists remotely, so a repeat would duplicate it: {error}"
    );
}

#[test]
fn a_rejected_create_is_retryable_and_a_lost_one_is_terminal() {
    for (status, terminal) in [(400, false), (401, false), (503, true)] {
        let server = MockServer::start();
        server.route(RequestKey::post(ISSUE), Route::Status(status));
        let client = client_for(&server, brief());

        let error = client
            .create("A title", "A body\n", "Bug")
            .expect_err("a non-2xx create fails");

        assert_eq!(
            matches!(error, TrackerError::Terminal { .. }),
            terminal,
            "status {status}: {error}"
        );
    }
}

#[test]
fn update_puts_the_whole_content_and_returns_nothing() {
    let server = MockServer::start();
    let key = RequestKey::put("/rest/api/3/issue/ENG-42");
    server.route(key.clone(), Route::Status(204));
    let client = client_for(&server, brief());

    client
        .update(&id("ENG-42"), "New title", "New body\n")
        .expect("update succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert_eq!(sent["fields"]["summary"], "New title");
    assert_eq!(sent["fields"]["description"]["type"], "doc");
}

#[test]
fn show_projects_the_body_with_exactly_one_trailing_newline() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/rest/api/3/issue/ENG-42"),
        Route::Json {
            status: 200,
            body: issue_payload("ENG-42", Some("2026-01-01T00:00:00.000+0000")),
        },
    );
    let client = client_for(&server, brief());

    let issue = client.show(&id("ENG-42")).expect("show succeeds");

    assert_eq!(issue.body, "A summary\n{\"type\":\"doc\",\"version\":1}\n");
    assert!(!issue.body.ends_with("\n\n"), "exactly one newline");
    assert_eq!(
        issue.updated,
        RemoteTimestamp::Reported("2026-01-01T00:00:00.000+0000".to_owned()),
        "the offset is held verbatim, colon-less as Jira sends it"
    );
}

#[test]
fn a_blank_absent_or_null_stamp_is_not_reported() {
    for stamp in ["null", "\"\""] {
        let server = MockServer::start();
        server.route(
            RequestKey::get("/rest/api/3/issue/ENG-1"),
            Route::Json {
                status: 200,
                body: format!(
                    "{{\"fields\":{{\"updated\":{stamp},\"summary\":\"S\"}}}}"
                ),
            },
        );
        let client = client_for(&server, brief());

        let issue = client.show(&id("ENG-1")).expect("show succeeds");

        assert_eq!(
            issue.updated,
            RemoteTimestamp::NotReported,
            "{stamp} must never become Reported(\"\")"
        );
    }
}

#[test]
fn a_404_on_show_is_retryable_not_an_absence() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/rest/api/3/issue/ENG-404"),
        Route::Status(404),
    );
    let client = client_for(&server, brief());

    let error = client
        .show(&id("ENG-404"))
        .expect_err("a 404 fails the read");

    assert!(
        matches!(error, TrackerError::Retryable { .. }),
        "a read never produces Terminal, and absence is not discoverable \
         through show: {error}"
    );
}

#[test]
fn an_empty_request_makes_no_remote_call() {
    let server = MockServer::start();
    let key = RequestKey::post(SEARCH);
    server.route(key.clone(), Route::Status(500));
    let client = client_for(&server, brief());

    let outcome = client.fetch_all(&[]).expect("an empty request succeeds");

    assert_eq!(server.hits(&key), 0, "key IN () would be malformed JQL");
    assert!(outcome.found.is_empty());
    assert!(outcome.absent.is_empty());
    assert!(outcome.indeterminate.is_empty());
}

#[test]
fn duplicate_ids_are_deduplicated_before_composition() {
    let server = MockServer::start();
    let key = RequestKey::post(SEARCH);
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: search_page(&["ENG-1"], None),
        },
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-1"), id("ENG-1")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert_eq!(sent["jql"], "key IN ('ENG-1')");
}

#[test]
fn the_key_clause_is_the_sole_filter() {
    let server = MockServer::start();
    let key = RequestKey::post(SEARCH);
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: search_page(&["ENG-1"], None),
        },
    );
    let client = client_for(&server, brief());

    client
        .fetch_all(&[id("ENG-1")])
        .expect("fetch_all succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    let jql = sent["jql"].as_str().expect("a jql string");
    assert!(
        !jql.contains("project ="),
        "an injected project clause would drop a cross-project key and the \
         sync would unlink a live issue: {jql}"
    );
    assert_eq!(sent["maxResults"], 100);
    assert_eq!(sent["fieldsByKeys"], false);
}

#[test]
fn a_hostile_identifier_leaves_one_bounded_clause() {
    let server = MockServer::start();
    let key = RequestKey::post(SEARCH);
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: search_page(&[], None),
        },
    );
    let client = client_for(&server, brief());

    client
        .fetch_all(&[id("X') OR project = FOO --")])
        .expect("fetch_all succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert_eq!(
        sent["jql"], "key IN ('X'') OR project = FOO --')",
        "the quote is doubled, as jira-jql.sh does, so the clause stays one \
         bounded literal"
    );
}

#[test]
fn an_unembeddable_identifier_is_a_preflight_error() {
    let server = MockServer::start();
    let key = RequestKey::post(SEARCH);
    server.route(key.clone(), Route::Status(200));
    let client = client_for(&server, brief());

    let error = client
        .fetch_all(&[id("ENG-1"), id("bad\nkey")])
        .expect_err("an unsafe id fails before any request");

    assert!(matches!(error, TrackerError::Retryable { .. }), "{error}");
    assert_eq!(server.hits(&key), 0);
}

#[test]
fn an_unfound_key_is_absent_when_the_retrieval_completed() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: search_page(&["ENG-1"], None),
        },
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-2")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
    assert_eq!(outcome.absent, vec![id("ENG-2")]);
    assert!(outcome.indeterminate.is_empty());
}

#[test]
fn a_failed_chunk_reports_its_keys_indeterminate_never_absent() {
    let server = MockServer::start();
    server.route(RequestKey::post(SEARCH), Route::Status(500));
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-2")])
        .expect("a transport-level failure is an Ok with the partition");

    assert!(
        outcome.absent.is_empty(),
        "inferring absence deletes live issues"
    );
    assert_eq!(outcome.indeterminate.len(), 2);
}

#[test]
fn the_cursor_is_followed_until_it_is_absent() {
    let server = MockServer::start();
    let key = RequestKey::post(SEARCH);
    server.route(
        key.clone(),
        Route::FlakyThenOk {
            fail_times: 0,
            body: search_page(&["ENG-1"], None).into_bytes(),
        },
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
    assert_eq!(server.hits(&key), 1, "no cursor means one page");
}

#[test]
fn a_stamp_absent_from_a_bulk_row_is_still_found() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(SEARCH),
        Route::Json {
            status: 200,
            body: "{\"issues\":[{\"key\":\"ENG-1\",\"fields\":{}}]}".to_owned(),
        },
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1")])
        .expect("fetch_all succeeds");

    assert_eq!(
        outcome.found,
        vec![(id("ENG-1"), RemoteTimestamp::NotReported)],
        "dropping a null-stamped row would report a live issue as deleted"
    );
}

#[test]
fn an_identifier_containing_a_slash_survives_encoding_and_validation() {
    let server = MockServer::start();
    let key = RequestKey::get("/rest/api/3/issue/OWNER%2FREPO-1");
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: issue_payload("OWNER/REPO-1", None),
        },
    );
    let client = client_for(&server, brief());

    client
        .show(&id("OWNER/REPO-1"))
        .expect("a legitimate slash is accepted end to end");

    assert_eq!(server.hits(&key), 1, "the segment is percent-encoded");
}

#[test]
fn a_traversal_bearing_identifier_is_refused_before_any_request() {
    let server = MockServer::start();
    let target = RequestKey::get("/rest/api/3/mypermissions");
    server.route(target.clone(), Route::Status(200));
    let client = client_for(&server, brief());

    for hostile in [
        "ENG/../../../../rest/api/3/mypermissions",
        "ENG%2f..%2f..%2fmypermissions",
        "ENG?x=",
    ] {
        let error = client
            .show(&id(hostile))
            .expect_err("a re-targeting identifier is refused");
        assert!(matches!(error, TrackerError::Retryable { .. }), "{error}");
    }
    assert_eq!(server.hits(&target), 0);
}

#[test]
fn the_page_cap_and_chunk_size_are_the_transcribed_ones() {
    let server = MockServer::start();
    let client: JiraClient = client_for(&server, brief());

    assert_eq!(client.transport().config().max_pages, 20);
}
