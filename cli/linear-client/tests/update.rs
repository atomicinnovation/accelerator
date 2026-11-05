//! An update sends exactly the fields it names, and nothing for an update that
//! names none.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockHTTPServer, RequestKey, Route};
use linear_client::classify::{GraphQlError, Outcome};
use linear_client::{LinearClient, LinearFailure, UpdateFields};
use serde_json::{json, Value};
use support::client::{brief, client_with, TEAM_KEY};
use tracker::ExternalId;

const GRAPHQL: &str = "/graphql";
const IDENTIFIER: &str = "ENG-1";

fn answering(status: u16, body: &str) -> (MockHTTPServer, LinearClient) {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Sequence(vec![Route::Json {
            status,
            body: body.to_owned(),
        }]),
    );
    let client =
        client_with(&server.base_url(), brief(), Some(TEAM_KEY.to_owned()));
    (server, client)
}

fn scripted_update() -> (MockHTTPServer, LinearClient) {
    answering(
        200,
        "{\"data\":{\"issueUpdate\":{\"success\":true,\
         \"issue\":{\"id\":\"u\",\"identifier\":\"ENG-1\"}}}}",
    )
}

fn issue() -> ExternalId {
    ExternalId::new(IDENTIFIER.to_owned())
}

fn sent_input(server: &MockHTTPServer) -> Value {
    let key = RequestKey::post(GRAPHQL);
    assert_eq!(server.hits(&key), 1, "exactly one issueUpdate is sent");
    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("an update"))
            .expect("a JSON request");
    assert_eq!(sent["variables"]["id"], json!(IDENTIFIER));
    sent["variables"]["input"].clone()
}

fn input_sent_for(fields: &UpdateFields<'_>) -> Value {
    let (server, client) = scripted_update();

    client
        .update_op(&issue(), fields)
        .expect("the update succeeds");

    sent_input(&server)
}

fn wire_outcome(failure: &LinearFailure) -> Outcome {
    match failure {
        LinearFailure::Wire { outcome, .. } => *outcome,
        LinearFailure::UnwritableIdentifier { .. } => {
            panic!("an update fails on the wire, never post-create")
        }
    }
}

#[test]
fn a_title_alone_is_the_whole_input() {
    let fields = UpdateFields {
        title: Some("Renamed"),
        ..UpdateFields::default()
    };

    assert_eq!(input_sent_for(&fields), json!({"title": "Renamed"}));
}

#[test]
fn a_description_alone_is_the_whole_input() {
    let fields = UpdateFields {
        description: Some("Body"),
        ..UpdateFields::default()
    };

    assert_eq!(input_sent_for(&fields), json!({"description": "Body"}));
}

#[test]
fn an_empty_description_is_sent_to_clear_it() {
    let fields = UpdateFields {
        description: Some(""),
        ..UpdateFields::default()
    };

    assert_eq!(input_sent_for(&fields), json!({"description": ""}));
}

#[test]
fn a_state_alone_is_the_whole_input() {
    let fields = UpdateFields {
        state_id: Some("state-ip-uuid"),
        ..UpdateFields::default()
    };

    assert_eq!(input_sent_for(&fields), json!({"stateId": "state-ip-uuid"}));
}

#[test]
fn an_assignee_alone_is_the_whole_input() {
    let fields = UpdateFields {
        assignee_id: Some("user-alice-uuid"),
        ..UpdateFields::default()
    };

    assert_eq!(
        input_sent_for(&fields),
        json!({"assigneeId": "user-alice-uuid"})
    );
}

#[test]
fn a_priority_alone_is_sent_as_a_number() {
    let fields = UpdateFields {
        priority: Some(1),
        ..UpdateFields::default()
    };

    assert_eq!(input_sent_for(&fields), json!({"priority": 1}));
}

#[test]
fn every_field_together_is_the_whole_input() {
    let fields = UpdateFields {
        title: Some("Renamed"),
        description: Some("Body"),
        state_id: Some("state-ip-uuid"),
        assignee_id: Some("user-alice-uuid"),
        priority: Some(2),
    };

    assert_eq!(
        input_sent_for(&fields),
        json!({
            "title": "Renamed",
            "description": "Body",
            "stateId": "state-ip-uuid",
            "assigneeId": "user-alice-uuid",
            "priority": 2,
        })
    );
}

#[test]
fn an_update_naming_no_field_is_refused_before_sending() {
    let (server, client) = scripted_update();

    let failure = client
        .update_op(&issue(), &UpdateFields::default())
        .expect_err("an empty update is refused");

    assert_eq!(wire_outcome(&failure), Outcome::RequestInvalid);
    assert_eq!(server.hits(&RequestKey::post(GRAPHQL)), 0);
}

#[test]
fn a_refused_identifier_is_refused_before_sending() {
    let (server, client) = scripted_update();
    let fields = UpdateFields {
        title: Some("Renamed"),
        ..UpdateFields::default()
    };

    let failure = client
        .update_op(&ExternalId::new("---ENG-1".to_owned()), &fields)
        .expect_err("the identifier is refused");

    assert_eq!(wire_outcome(&failure), Outcome::RequestInvalid);
    assert_eq!(server.hits(&RequestKey::post(GRAPHQL)), 0);
}

#[test]
fn a_validation_error_is_a_bad_request_despite_the_200() {
    let (_server, client) = answering(
        200,
        "{\"data\":null,\"errors\":[{\"message\":\"Argument Validation \
         Error\",\"extensions\":{\"code\":\"INVALID_INPUT\"}}]}",
    );
    let fields = UpdateFields {
        priority: Some(9),
        ..UpdateFields::default()
    };

    let failure = client
        .update_op(&issue(), &fields)
        .expect_err("the update is refused");

    assert_eq!(
        wire_outcome(&failure),
        Outcome::SuccessWithErrors(GraphQlError::BadRequest)
    );
}
