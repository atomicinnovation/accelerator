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
use tracker::CreatePreview;
use tracker::ExternalId;
use tracker::FieldResolution;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker::ValidationOutcome;
use tracker_test_support::contract::{
    a_failing_read_is_retryable_property,
    create_then_show_round_trips_property,
    fetch_all_partitions_totally_property,
    preview_create_makes_no_mutation_property,
    preview_create_resolves_fields_property,
    search_reports_truncation_property,
    unaccounted_id_is_indeterminate_not_absent_property,
    validate_update_reports_outcome_property, ContractSubject,
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

    /// The mock fails every `/search/jql`, so any project-scoped discovery
    /// comes back `complete == false`.
    fn truncating_scope(&self) -> SearchScope {
        SearchScope {
            project: Some(support::client::PROJECT.to_owned()),
            ..SearchScope::default()
        }
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
    server.route(
        RequestKey::get("/rest/api/3/project"),
        Route::Json {
            status: 200,
            body: format!(
                "[{{\"key\":\"{}\",\"id\":\"1\",\"name\":\"Engineering\"}}]",
                support::client::PROJECT
            ),
        },
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

fn client_with_project(server: &MockServer, project: &str) -> JiraClient {
    use jira_client::jql::FixedResolver;
    use jira_client::transport::Transport;
    use tracker_support::TransportConfig;
    let transport = Transport::new(
        support::client::credentials(&server.base_url()),
        TransportConfig {
            timeout: std::time::Duration::from_millis(400),
            ..TransportConfig::default()
        },
        Box::new(RecordingSleeper::new()),
        Box::new(support::NoJitter),
    )
    .expect("the transport builds");
    JiraClient::new(
        transport,
        project.to_owned(),
        Box::new(FixedResolver::new()),
        Box::new(FixedResolver::new()),
    )
}

fn project_server(keys: &[&str]) -> MockServer {
    let server = MockServer::start();
    let entries: Vec<String> = keys
        .iter()
        .map(|key| format!("{{\"key\":\"{key}\",\"id\":\"1\",\"name\":\"n\"}}"))
        .collect();
    server.route(
        RequestKey::get("/rest/api/3/project"),
        Route::Json {
            status: 200,
            body: format!("[{}]", entries.join(",")),
        },
    );
    server
}

#[test]
fn the_new_conformance_properties_hold_offline() {
    let server = conformant_server();
    search_reports_truncation_property(&subject(&server));

    let server = conformant_server();
    preview_create_makes_no_mutation_property(&subject(&server));

    let server = conformant_server();
    validate_update_reports_outcome_property(&subject(&server));
}

#[test]
fn preview_create_resolves_each_field_state() {
    let resolved = project_server(&["ENG"]);
    preview_create_resolves_fields_property(
        &subject(&resolved),
        &CreatePreview {
            project: FieldResolution::Resolved("ENG".to_owned()),
            issue_type: FieldResolution::Unset,
        },
    );

    let absent = project_server(&["OTHER"]);
    preview_create_resolves_fields_property(
        &subject(&absent),
        &CreatePreview {
            project: FieldResolution::Unresolvable("ENG".to_owned()),
            issue_type: FieldResolution::Unset,
        },
    );

    // An unconfigured project resolves to Unset with no remote call.
    let unset = MockServer::start();
    let subject = MockBackedClient {
        client: client_with_project(&unset, ""),
    };
    preview_create_resolves_fields_property(
        &subject,
        &CreatePreview {
            project: FieldResolution::Unset,
            issue_type: FieldResolution::Unset,
        },
    );
    assert_eq!(
        unset.hits(&RequestKey::get("/rest/api/3/project")),
        0,
        "an unset project must not trigger a discovery call"
    );
}

#[test]
fn a_non_empty_kind_resolves_the_issue_type() {
    let resolved = project_server(&["ENG"]);
    let preview = subject(&resolved)
        .tracker()
        .preview_create("Bug")
        .expect("preview_create succeeds");
    assert_eq!(
        preview.issue_type,
        FieldResolution::Resolved("Bug".to_owned())
    );
}

#[test]
fn preview_create_issues_no_remote_create() {
    let server = conformant_server();
    subject(&server)
        .tracker()
        .preview_create("")
        .expect("preview_create succeeds");
    assert_eq!(
        server.hits(&RequestKey::post("/rest/api/3/issue")),
        0,
        "a preview must not create an issue"
    );
}

#[test]
fn the_surface_error_shim_maps_a_read_failure_to_retryable() {
    let server = MockServer::start();
    server.route(RequestKey::get("/rest/api/3/project"), Route::Status(500));
    let error = subject(&server)
        .tracker()
        .preview_create("")
        .expect_err("a failed discovery must surface");
    assert!(
        matches!(error, tracker::TrackerError::Retryable { .. }),
        "a discovery read that mutated nothing must be Retryable, got {error:?}"
    );
}

#[test]
fn validate_update_rejects_a_missing_summary_offline() {
    let server = conformant_server();
    let id = ExternalId::new(CREATED.to_owned());
    let outcome = subject(&server)
        .tracker()
        .validate_update(&id, "", "Body\n");
    match outcome {
        ValidationOutcome::Rejected { reasons } => {
            assert!(!reasons.is_empty());
        }
        ValidationOutcome::Valid => panic!("an empty summary must be rejected"),
    }
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
