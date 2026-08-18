//! The port's contract properties against a mock, in the **default** profile —
//! the enforcing route, with the live harness as additional assurance.
//!
//! Linear posts every operation to one endpoint, so the mock is scripted with
//! `Route::Sequence` rather than keyed per operation: a `(method, path)` key
//! cannot tell a create from the `show` that follows it.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::LinearClient;
use support::client::{brief, client_with, TEAM_KEY};
use tracker::ExternalId;
use tracker::RemoteTracker;
use tracker_test_support::contract::{
    a_failing_read_is_retryable_property,
    create_then_show_round_trips_property,
    fetch_all_partitions_totally_property, partitions_totally,
    unaccounted_id_is_indeterminate_not_absent_property, ContractSubject,
};

const GRAPHQL: &str = "/graphql";
const CREATED: &str = "ENG-1";
/// Another team's identifier: in scope for no search this client can run, so
/// its absence is never provable.
const UNACCOUNTABLE: &str = "OPS-999";
const UNREADABLE: &str = "ENG-404";

struct MockBackedClient {
    client: LinearClient,
}

impl ContractSubject for MockBackedClient {
    fn tracker(&self) -> &dyn RemoteTracker {
        &self.client
    }

    fn unaccountable_id(&self) -> ExternalId {
        ExternalId::new(UNACCOUNTABLE.to_owned())
    }

    fn unreadable_id(&self) -> ExternalId {
        ExternalId::new(UNREADABLE.to_owned())
    }
}

fn json(body: &str) -> Route {
    Route::Json {
        status: 200,
        body: body.to_owned(),
    }
}

fn created() -> Route {
    json(
        "{\"data\":{\"issueCreate\":{\"success\":true,\
         \"issue\":{\"id\":\"u\",\"identifier\":\"ENG-1\"}}}}",
    )
}

fn shown(title: &str, description: &str) -> Route {
    json(&format!(
        "{{\"data\":{{\"issue\":{{\"id\":\"u\",\"identifier\":\"ENG-1\",\
         \"title\":\"{title}\",\"updatedAt\":\"2026-01-01T00:00:00.000Z\",\
         \"description\":\"{description}\"}}}}}}"
    ))
}

fn searched() -> Route {
    json(
        "{\"data\":{\"issues\":{\"nodes\":[{\"identifier\":\"ENG-1\",\
         \"updatedAt\":\"2026-01-01T00:00:00.000Z\"}],\
         \"pageInfo\":{\"hasNextPage\":false}}}}",
    )
}

fn scripted(responses: Vec<Route>) -> (MockServer, MockBackedClient) {
    let server = MockServer::start();
    server.route(RequestKey::post(GRAPHQL), Route::Sequence(responses));
    let client =
        client_with(&server.base_url(), brief(), Some(TEAM_KEY.to_owned()));
    (server, MockBackedClient { client })
}

#[test]
fn the_conformance_properties_hold_offline() {
    let mut executed = 0;

    let (_server, subject) =
        scripted(vec![created(), shown("Contract title", "Contract body")]);
    create_then_show_round_trips_property(&subject);
    executed += 1;

    // The search completes and does not carry OPS-999: another team's
    // identifier, which this client never had scope to see.
    let (_server, subject) = scripted(vec![searched()]);
    unaccounted_id_is_indeterminate_not_absent_property(&subject);
    executed += 1;

    let (_server, subject) = scripted(vec![Route::Status(500)]);
    a_failing_read_is_retryable_property(&subject);
    executed += 1;

    let (_server, subject) = scripted(vec![searched()]);
    let ids = [
        ExternalId::new(CREATED.to_owned()),
        ExternalId::new(UNACCOUNTABLE.to_owned()),
    ];
    fetch_all_partitions_totally_property(&subject, &ids);
    executed += 1;

    assert_eq!(
        executed, 4,
        "a regression that made every property a no-op must be \
         distinguishable from a real run"
    );
}

#[test]
fn update_replaces_the_content_offline() {
    let (_server, subject) = scripted(vec![
        created(),
        json(
            "{\"data\":{\"issueUpdate\":{\"success\":true,\
             \"issue\":{\"identifier\":\"ENG-1\"}}}}",
        ),
    ]);

    let created = subject
        .tracker()
        .create("Original", "Original body\n", "")
        .expect("create succeeds");
    subject
        .tracker()
        .update(&created, "Updated", "Updated body\n")
        .expect("update succeeds");
}

#[test]
fn the_partition_stays_total_over_a_mixed_request() {
    let (_server, subject) = scripted(vec![searched()]);
    let ids = [
        ExternalId::new(CREATED.to_owned()),
        ExternalId::new("ENG-2".to_owned()),
        ExternalId::new(UNACCOUNTABLE.to_owned()),
    ];

    let outcome = subject
        .tracker()
        .fetch_all(&ids)
        .expect("fetch_all succeeds");

    partitions_totally(&outcome, &ids);
    assert_eq!(
        outcome.absent,
        vec![ExternalId::new("ENG-2".to_owned())],
        "an in-team id missing from a complete search is provably absent"
    );
    assert_eq!(
        outcome.indeterminate,
        vec![ExternalId::new(UNACCOUNTABLE.to_owned())],
        "another team's id is not"
    );
}
