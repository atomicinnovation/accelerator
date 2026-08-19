//! Comment creation: the exact mutation document and variables, and the
//! failure surface.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::SurfaceError;
use serde_json::Value;
use support::client::client_for;
use tracker_support::TransportConfig;

const GRAPHQL: &str = "/graphql";

fn json_route(body: &str) -> Route {
    Route::Json {
        status: 200,
        body: body.to_owned(),
    }
}

#[test]
fn add_comment_sends_the_mutation_with_the_issue_id_and_markdown_body() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        json_route(
            "{\"data\":{\"commentCreate\":{\"success\":true,\
             \"comment\":{\"id\":\"c1\"}}}}",
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    client
        .add_comment("ENG-1", "A *markdown* comment")
        .expect("the comment is added");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert!(sent["query"]
        .as_str()
        .expect("a document")
        .contains("commentCreate"));
    assert_eq!(sent["variables"]["input"]["issueId"], "ENG-1");
    assert_eq!(
        sent["variables"]["input"]["body"], "A *markdown* comment",
        "Linear comments are Markdown-native: the body passes through verbatim"
    );
}

#[test]
fn a_response_carrying_errors_is_a_typed_failure() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route("{\"errors\":[{\"message\":\"nope\"}]}"),
    );
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .add_comment("ENG-1", "body")
        .expect_err("a 200 carrying errors is a failure");

    assert!(
        matches!(error, SurfaceError::GraphQlErrors { .. }),
        "{error}"
    );
}
