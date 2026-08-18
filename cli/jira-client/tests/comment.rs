//! Comment request construction: each shape's method, path, body and query,
//! and the list pagination's 20-page cap.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::comment::Visibility;
use jira_client::SurfaceError;
use serde_json::Value;
use support::client::client_for;
use tracker_support::TransportConfig;

const KEY: &str = "ENG-1";
const COMMENT_PATH: &str = "/rest/api/3/issue/ENG-1/comment";
const ONE_COMMENT: &str = "/rest/api/3/issue/ENG-1/comment/10001";

fn body_json(server: &MockServer, key: &RequestKey) -> Value {
    let bytes = server.last_body(key).expect("a recorded body");
    serde_json::from_slice(&bytes).expect("the body is JSON")
}

#[test]
fn add_posts_the_adf_body_with_no_query_when_notifying() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(COMMENT_PATH),
        Route::Json {
            status: 201,
            body: r#"{"id":"10001"}"#.to_owned(),
        },
    );
    let client = client_for(&server, TransportConfig::default());

    client
        .add_comment(KEY, "hello", None, true)
        .expect("the comment is added");

    let key = RequestKey::post(COMMENT_PATH);
    assert_eq!(server.hits(&key), 1);
    let body = body_json(&server, &key);
    assert_eq!(body["body"]["type"], "doc");
    assert!(body.get("visibility").is_none());
    assert_eq!(server.last_query(&key), None);
}

#[test]
fn add_carries_visibility_and_the_no_notify_query() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(COMMENT_PATH),
        Route::Json {
            status: 201,
            body: "{}".to_owned(),
        },
    );
    let client = client_for(&server, TransportConfig::default());

    client
        .add_comment(
            KEY,
            "hi",
            Some(&Visibility::Role("Administrators".to_owned())),
            false,
        )
        .expect("the comment is added");

    let key = RequestKey::post(COMMENT_PATH);
    let body = body_json(&server, &key);
    assert_eq!(body["visibility"]["type"], "role");
    assert_eq!(body["visibility"]["value"], "Administrators");
    assert_eq!(
        server.last_query(&key).as_deref(),
        Some("notifyUsers=false")
    );
}

#[test]
fn edit_puts_to_the_comment_id_path() {
    let server = MockServer::start();
    server.route(
        RequestKey::put(ONE_COMMENT),
        Route::Json {
            status: 200,
            body: r#"{"id":"10001"}"#.to_owned(),
        },
    );
    let client = client_for(&server, TransportConfig::default());

    client
        .edit_comment(KEY, "10001", "revised", None, true)
        .expect("the comment is edited");

    let key = RequestKey::put(ONE_COMMENT);
    assert_eq!(server.hits(&key), 1);
    assert_eq!(body_json(&server, &key)["body"]["type"], "doc");
}

#[test]
fn delete_sends_delete_and_honours_no_notify() {
    let server = MockServer::start();
    server.route(RequestKey::new("DELETE", ONE_COMMENT), Route::Status(204));
    let client = client_for(&server, TransportConfig::default());

    client
        .delete_comment(KEY, "10001", false)
        .expect("the comment is deleted");

    let key = RequestKey::new("DELETE", ONE_COMMENT);
    assert_eq!(server.hits(&key), 1);
    assert_eq!(
        server.last_query(&key).as_deref(),
        Some("notifyUsers=false")
    );
}

#[test]
fn list_follows_offset_pagination_to_the_end() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(COMMENT_PATH),
        Route::Sequence(vec![
            Route::Json {
                status: 200,
                body: r#"{"total":3,"comments":[{"id":"1"},{"id":"2"}]}"#
                    .to_owned(),
            },
            Route::Json {
                status: 200,
                body: r#"{"total":3,"comments":[{"id":"3"}]}"#.to_owned(),
            },
        ]),
    );
    let client = client_for(&server, TransportConfig::default());

    let page = client
        .list_comments(KEY, 2, false)
        .expect("the list returns");

    assert_eq!(server.hits(&RequestKey::get(COMMENT_PATH)), 2);
    assert_eq!(page.total, 3);
    assert!(!page.truncated);
    assert_eq!(page.comments.len(), 3);
}

#[test]
fn list_stops_at_twenty_pages_and_reports_truncation() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(COMMENT_PATH),
        Route::Json {
            status: 200,
            body: r#"{"total":1000,"comments":[{"id":"x"}]}"#.to_owned(),
        },
    );
    let client = client_for(&server, TransportConfig::default());

    let page = client
        .list_comments(KEY, 1, false)
        .expect("the list returns");

    assert_eq!(server.hits(&RequestKey::get(COMMENT_PATH)), 20);
    assert!(page.truncated);
    assert_eq!(page.comments.len(), 20);
}

#[test]
fn list_first_page_only_makes_one_request() {
    let server = MockServer::start();
    server.route(
        RequestKey::get(COMMENT_PATH),
        Route::Json {
            status: 200,
            body: r#"{"total":1000,"comments":[{"id":"x"}]}"#.to_owned(),
        },
    );
    let client = client_for(&server, TransportConfig::default());

    let page = client
        .list_comments(KEY, 50, true)
        .expect("the list returns");

    assert_eq!(server.hits(&RequestKey::get(COMMENT_PATH)), 1);
    assert!(!page.truncated);
}

#[test]
fn a_page_size_out_of_range_is_refused_before_any_request() {
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let error = client.list_comments(KEY, 0, false).expect_err("refused");

    assert!(matches!(error, SurfaceError::BadPageSize { got: 0 }));
    assert_eq!(server.hits(&RequestKey::get(COMMENT_PATH)), 0);
}

#[test]
fn a_non_success_status_surfaces_as_a_status_error() {
    let server = MockServer::start();
    server.route(RequestKey::post(COMMENT_PATH), Route::Status(403));
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .add_comment(KEY, "hi", None, true)
        .expect_err("a 403 is an error");

    assert!(matches!(
        error,
        SurfaceError::Status {
            status: 403,
            operation: "comment add",
            ..
        }
    ));
}
