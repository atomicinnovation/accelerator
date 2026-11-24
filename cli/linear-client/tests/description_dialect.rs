//! A description reaches Linear in Linear's Markdown dialect: escaped so its
//! parser reads it as `CommonMark` does, unless it is sent verbatim.

#![allow(clippy::expect_used)]

mod support;

use http_test_support::{MockHTTPServer, RequestKey, Route};
use linear_client::{Description, LinearClient, UpdateFields};
use serde_json::Value;
use support::client::{brief, client_for};
use tracker::{ExternalId, RemoteTracker as _};

const GRAPHQL: &str = "/graphql";
const NEEDS_BOTH_ESCAPES: &str = "- owned by\n  0160.\n\nSee crates.io.\n";
const ESCAPED: &str = "- owned by\n  0160\\.\n\nSee crates\\.io.\n";
const CREATED: &str = "{\"data\":{\"issueCreate\":{\"success\":true,\
     \"issue\":{\"id\":\"u\",\"identifier\":\"ENG-42\"}}}}";
const UPDATED: &str = "{\"data\":{\"issueUpdate\":{\"success\":true,\
     \"issue\":{\"id\":\"u\",\"identifier\":\"ENG-1\"}}}}";

fn answering(body: &str) -> (MockHTTPServer, LinearClient) {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Json {
            status: 200,
            body: body.to_owned(),
        },
    );
    let client = client_for(&server, brief());
    (server, client)
}

fn issue() -> ExternalId {
    ExternalId::new("ENG-1".to_owned())
}

fn sent_input(server: &MockHTTPServer) -> Value {
    let sent: Value = serde_json::from_slice(
        &server
            .last_body(&RequestKey::post(GRAPHQL))
            .expect("a request"),
    )
    .expect("a JSON request");
    sent["variables"]["input"].clone()
}

fn sent_description(server: &MockHTTPServer) -> String {
    sent_input(server)["description"]
        .as_str()
        .expect("a description")
        .to_owned()
}

fn description_created_from(description: Description<'_>) -> String {
    let (server, client) = answering(CREATED);
    client
        .create_op("A title", description, "story")
        .expect("the create succeeds");
    sent_description(&server)
}

fn description_updated_to(description: Description<'_>) -> String {
    let (server, client) = answering(UPDATED);
    client
        .update_op(
            &issue(),
            &UpdateFields {
                description: Some(description),
                ..UpdateFields::default()
            },
        )
        .expect("the update succeeds");
    sent_description(&server)
}

#[test]
fn create_op_escapes_a_markdown_description() {
    assert_eq!(
        description_created_from(Description::Markdown(NEEDS_BOTH_ESCAPES)),
        ESCAPED
    );
}

#[test]
fn update_op_escapes_a_markdown_description() {
    assert_eq!(
        description_updated_to(Description::Markdown(NEEDS_BOTH_ESCAPES)),
        ESCAPED
    );
}

#[test]
fn create_op_and_update_op_send_a_verbatim_description_as_given() {
    assert_eq!(
        description_created_from(Description::Verbatim(NEEDS_BOTH_ESCAPES)),
        NEEDS_BOTH_ESCAPES
    );
    assert_eq!(
        description_updated_to(Description::Verbatim(NEEDS_BOTH_ESCAPES)),
        NEEDS_BOTH_ESCAPES
    );
}

#[test]
fn remote_tracker_create_and_update_send_the_escaped_body() {
    let (created, client) = answering(CREATED);
    client
        .create("A title", NEEDS_BOTH_ESCAPES, "story")
        .expect("the create succeeds");
    assert_eq!(sent_description(&created), ESCAPED);

    let (updated, client) = answering(UPDATED);
    client
        .update(&issue(), Some("A title"), NEEDS_BOTH_ESCAPES)
        .expect("the update succeeds");
    assert_eq!(sent_description(&updated), ESCAPED);
}

#[test]
fn retitle_sends_no_description() {
    let (server, client) = answering(UPDATED);
    client
        .retitle(&issue(), "A title")
        .expect("the retitle succeeds");
    assert!(sent_input(&server).get("description").is_none());
}
