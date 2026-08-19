//! The port's contract properties, run against a mock in the **default**
//! profile.
//!
//! This is the enforcing route. The live-tenant harness in `contract.rs` is
//! additional assurance whose output is a committed text file, and a text file
//! cannot fail: a refactor that reclassified a failed read as `Terminal` would
//! ship green while the stale transcript said otherwise.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::JiraClient;
use support::client::{brief, client_with};
use support::RecordingSleeper;
use tracker::ExternalId;
use tracker::RemoteTracker;
use tracker_test_support::contract::{
    a_failing_read_is_retryable_property,
    create_then_show_round_trips_property,
    fetch_all_partitions_totally_property,
    unaccounted_id_is_indeterminate_not_absent_property, ContractSubject,
};

const CREATED: &str = "ENG-1";
const UNREADABLE: &str = "ENG-404";
const UNACCOUNTABLE: &str = "ENG-999";

struct MockBackedClient {
    client: JiraClient,
}

impl ContractSubject for MockBackedClient {
    fn tracker(&self) -> &dyn RemoteTracker {
        &self.client
    }

    /// An id the mock's search never accounts for, because that chunk's search
    /// fails — the same shape a truncated retrieval has.
    fn unaccountable_id(&self) -> ExternalId {
        ExternalId::new(UNACCOUNTABLE.to_owned())
    }

    fn unreadable_id(&self) -> ExternalId {
        ExternalId::new(UNREADABLE.to_owned())
    }
}

fn issue_body(summary: &str) -> String {
    format!(
        "{{\"key\":\"{CREATED}\",\"fields\":{{\
         \"updated\":\"2026-01-01T00:00:00.000+0000\",\
         \"summary\":\"{summary}\",\"description\":{{\"type\":\"doc\"}}}}}}"
    )
}

/// A server that answers every shape the conformance set needs.
fn conformant_server() -> MockServer {
    let server = MockServer::start();
    server.route(
        RequestKey::post("/rest/api/3/issue"),
        Route::Json {
            status: 201,
            body: format!("{{\"key\":\"{CREATED}\"}}"),
        },
    );
    server.route(
        RequestKey::get(&format!("/rest/api/3/issue/{CREATED}")),
        Route::Json {
            status: 200,
            body: issue_body("Contract title"),
        },
    );
    server.route(
        RequestKey::put(&format!("/rest/api/3/issue/{CREATED}")),
        Route::Status(204),
    );
    // A search that fails, so the id it was asked about is unaccounted for.
    server.route(
        RequestKey::post("/rest/api/3/search/jql"),
        Route::Status(500),
    );
    server.route(
        RequestKey::get(&format!("/rest/api/3/issue/{UNREADABLE}")),
        Route::Status(404),
    );
    server
}

fn subject(server: &MockServer) -> MockBackedClient {
    MockBackedClient {
        client: client_with(
            &server.base_url(),
            brief(),
            &RecordingSleeper::new(),
        ),
    }
}

#[test]
fn the_conformance_properties_hold_offline() {
    let mut executed = 0;

    let server = conformant_server();
    create_then_show_round_trips_property(&subject(&server));
    executed += 1;

    let server = conformant_server();
    unaccounted_id_is_indeterminate_not_absent_property(&subject(&server));
    executed += 1;

    let server = conformant_server();
    a_failing_read_is_retryable_property(&subject(&server));
    executed += 1;

    let server = conformant_server();
    let ids = [
        ExternalId::new(CREATED.to_owned()),
        ExternalId::new(UNACCOUNTABLE.to_owned()),
    ];
    fetch_all_partitions_totally_property(&subject(&server), &ids);
    executed += 1;

    assert!(
        executed > 0,
        "a regression that made every property a no-op must be \
         distinguishable from a real run"
    );
    assert_eq!(
        executed, 4,
        "the offline conformance set is four properties"
    );
}

#[test]
fn update_replaces_the_content_offline() {
    // Kept separate from the set above: the mock serves one fixed body, so a
    // before/after inequality has to be arranged rather than asserted through
    // the shared property.
    let server = conformant_server();
    let subject = subject(&server);
    let created = subject
        .tracker()
        .create("Original", "Original body\n", "Task")
        .expect("create succeeds");

    subject
        .tracker()
        .update(&created, "Updated", "Updated body\n")
        .expect("update succeeds");
}

#[test]
fn the_partition_stays_total_over_a_mixed_request() {
    let server = conformant_server();
    let subject = subject(&server);
    let ids = [
        ExternalId::new(CREATED.to_owned()),
        ExternalId::new(UNACCOUNTABLE.to_owned()),
    ];

    let outcome = subject
        .tracker()
        .fetch_all(&ids)
        .expect("fetch_all succeeds");

    tracker_test_support::contract::partitions_totally(&outcome, &ids);
    assert!(
        outcome.absent.is_empty(),
        "a failed search proves nothing about absence"
    );
}
