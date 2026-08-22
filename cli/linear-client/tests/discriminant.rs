//! The structured discriminant the `*_op` methods surface (Decision 9): the
//! binary reads the granular bash exit code from it, and the post-create
//! "created remotely but unwritable" case is a distinct variant, not a wire
//! outcome — so it never has to be parsed back out of a `TrackerError` string.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::classify::bash_code;
use linear_client::LinearFailure;
use support::client::{brief, client_for};
use tracker::ExternalId;

const GRAPHQL: &str = "/graphql";

fn id(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

fn json(status: u16, body: &str) -> Route {
    Route::Json {
        status,
        body: body.to_owned(),
    }
}

#[test]
fn a_create_wire_failure_surfaces_the_outcome_the_exit_code_reads() {
    let server = MockServer::start();
    server.route(RequestKey::post(GRAPHQL), json(401, "{}"));

    let client = client_for(&server, brief());
    let failure = client.create_op("t", "b", "story").expect_err("401");

    match failure {
        LinearFailure::Wire { outcome, .. } => {
            assert_eq!(bash_code(outcome), 11, "401 is E_GQL_UNAUTHORIZED");
        }
        LinearFailure::UnwritableIdentifier { .. } => {
            panic!("a 401 is a wire failure, not the post-create case")
        }
    }
}

#[test]
fn a_created_but_unwritable_identifier_is_a_distinct_variant() {
    let server = MockServer::start();
    // The mutation succeeds, but the returned identifier carries a control
    // byte the frontmatter-safety check refuses — created remotely, unwritable.
    let body = "{\"data\":{\"issueCreate\":{\"issue\":\
                {\"id\":\"u\",\"identifier\":\"ENG-1\\u0001\"}}}}";
    server.route(RequestKey::post(GRAPHQL), json(200, body));

    let client = client_for(&server, brief());
    let failure = client.create_op("t", "b", "story").expect_err("unwritable");

    assert!(
        matches!(failure, LinearFailure::UnwritableIdentifier { .. }),
        "expected the post-create case, got {failure:?}"
    );
}

#[test]
fn a_show_wire_failure_carries_the_granular_ratelimit_code() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json(400, r#"{"errors":[{"extensions":{"code":"RATELIMITED"}}]}"#),
    );

    let client = client_for(&server, brief());
    let failure = client.show_op(&id("ENG-1")).expect_err("ratelimited");

    match failure {
        LinearFailure::Wire { outcome, .. } => {
            assert_eq!(bash_code(outcome), 35, "a 400 ratelimit is 35");
        }
        LinearFailure::UnwritableIdentifier { .. } => {
            panic!("a ratelimit is a wire failure, not the post-create case")
        }
    }
}
