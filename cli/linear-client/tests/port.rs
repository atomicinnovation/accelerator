//! The four port operations, against a mock server.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::sync::{Arc, Mutex};

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::catalogue::{Catalogue, LiveCatalogueData};
use linear_client::healing::CatalogueBackfill;
use linear_client::resolution::ResolverSet;
use serde_json::{json, Value};
use support::catalogue::{complete_entry, entry, label, resolvers, state};
use support::client::{
    brief, client_for, client_holding_into, client_with, client_with_resolvers,
    client_with_teams, TEAM_ID, TEAM_KEY,
};
use tracker::{
    Ceiling, ExternalId, RemoteTimestamp, RemoteTracker as _, SearchScope,
    TrackerError,
};
use tracker_support::TransportConfig;

const GRAPHQL: &str = "/graphql";

/// A `TransportConfig` with `brief`'s short timeout and the given page caps.
fn caps(discovery: Ceiling, keyed_read: Ceiling) -> TransportConfig {
    TransportConfig {
        discovery_max_pages: discovery,
        keyed_read_max_pages: keyed_read,
        ..brief()
    }
}

/// Three search pages, the first two cursored so only a page cap or the third
/// (cursorless) page stops the walk.
fn three_pages() -> Route {
    Route::Sequence(vec![
        json_route(search_body(&["ENG-1"], Some("p2"))),
        json_route(search_body(&["ENG-2"], Some("p3"))),
        json_route(search_body(&["ENG-3"], None)),
    ])
}

fn id(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

const fn json_route(body: String) -> Route {
    Route::Json { status: 200, body }
}

fn issue_body(identifier: &str, updated: &str, description: &str) -> String {
    format!(
        "{{\"data\":{{\"issue\":{{\"id\":\"uuid\",\
         \"identifier\":\"{identifier}\",\"title\":\"A title\",\
         \"updatedAt\":\"{updated}\",\"description\":{description}}}}}}}"
    )
}

fn search_body(identifiers: &[&str], next: Option<&str>) -> String {
    let nodes: Vec<String> = identifiers
        .iter()
        .map(|identifier| {
            format!(
                "{{\"id\":\"u\",\"identifier\":\"{identifier}\",\
                 \"title\":\"t\",\"updatedAt\":\"2026-01-01T00:00:00.000Z\"}}"
            )
        })
        .collect();
    let page = next.map_or_else(
        || "{\"hasNextPage\":false,\"endCursor\":null}".to_owned(),
        |cursor| format!("{{\"hasNextPage\":true,\"endCursor\":\"{cursor}\"}}"),
    );
    format!(
        "{{\"data\":{{\"issues\":{{\"nodes\":[{}],\"pageInfo\":{page}}}}}}}",
        nodes.join(",")
    )
}

#[test]
fn create_sends_the_mutation_and_returns_the_identifier() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        json_route(
            "{\"data\":{\"issueCreate\":{\"success\":true,\
             \"issue\":{\"id\":\"u\",\"identifier\":\"ENG-42\"}}}}"
                .to_owned(),
        ),
    );
    let client = client_for(&server, brief());

    let created = client
        .create("A title", "A body\n", "story")
        .expect("create succeeds");

    assert_eq!(created, id("ENG-42"));
    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert!(sent["query"]
        .as_str()
        .expect("a document")
        .contains("issueCreate"));
    assert_eq!(sent["variables"]["input"]["teamId"], TEAM_ID);
    assert_eq!(sent["variables"]["input"]["title"], "A title");
    assert_eq!(
        sent["variables"]["input"]["description"], "A body\n",
        "Linear is Markdown-native: the body passes through verbatim"
    );
}

#[test]
fn a_created_identifier_that_cannot_be_written_back_is_terminal() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(
            "{\"data\":{\"issueCreate\":{\"issue\":{\"identifier\":\"---X\"}}}}"
                .to_owned(),
        ),
    );
    let client = client_for(&server, brief());

    let error = client
        .create("A title", "A body\n", "")
        .expect_err("an unusable identifier is a failure");

    assert!(matches!(error, TrackerError::Terminal { .. }), "{error}");
}

#[test]
fn a_two_hundred_carrying_an_auth_error_is_retryable_on_create_and_update() {
    let body = "{\"errors\":[{\"message\":\"no\",\
                \"extensions\":{\"type\":\"authentication error\"}}]}";
    for expectation in ["create", "update"] {
        let server = MockServer::start();
        server.route(RequestKey::post(GRAPHQL), json_route(body.to_owned()));
        let client = client_for(&server, brief());

        let error = if expectation == "create" {
            client.create("t", "b\n", "").expect_err("it fails")
        } else {
            client
                .update(&id("ENG-1"), "t", "b\n")
                .expect_err("it fails")
        };

        assert!(
            matches!(error, TrackerError::Retryable { .. }),
            "{expectation}: a provably-unapplied auth rejection: {error}"
        );
    }
}

#[test]
fn a_two_hundred_carrying_an_unclassified_error_diverges_between_operations() {
    let body = "{\"errors\":[{\"message\":\"Field does not exist\"}]}";

    let server = MockServer::start();
    server.route(RequestKey::post(GRAPHQL), json_route(body.to_owned()));
    let created = client_for(&server, brief())
        .create("t", "b\n", "")
        .expect_err("it fails");
    assert!(
        matches!(created, TrackerError::Retryable { .. }),
        "{created}"
    );

    let server = MockServer::start();
    server.route(RequestKey::post(GRAPHQL), json_route(body.to_owned()));
    let updated = client_for(&server, brief())
        .update(&id("ENG-1"), "t", "b\n")
        .expect_err("it fails");
    assert!(
        matches!(updated, TrackerError::Terminal { .. }),
        "a 200-body error may mean the update applied: {updated}"
    );
}

#[test]
fn update_sends_the_mutation_with_the_identifier_and_input() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        json_route(
            "{\"data\":{\"issueUpdate\":{\"success\":true,\
             \"issue\":{\"identifier\":\"ENG-1\"}}}}"
                .to_owned(),
        ),
    );
    let client = client_for(&server, brief());

    client
        .update(&id("ENG-1"), "New title", "New body\n")
        .expect("update succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert!(sent["query"]
        .as_str()
        .expect("a document")
        .contains("issueUpdate"));
    assert_eq!(sent["variables"]["id"], "ENG-1");
    assert_eq!(sent["variables"]["input"]["title"], "New title");
    assert_eq!(sent["variables"]["input"]["description"], "New body\n");
}

#[test]
fn show_projects_the_body_with_exactly_one_trailing_newline() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(issue_body(
            "ENG-1",
            "2026-01-01T00:00:00.000Z",
            "\"Some *markdown*\"",
        )),
    );
    let client = client_for(&server, brief());

    let issue = client.show(&id("ENG-1")).expect("show succeeds");

    assert_eq!(issue.body, "A title\nSome *markdown*\n");
    assert!(!issue.body.ends_with("\n\n"));
    assert_eq!(
        issue.updated,
        RemoteTimestamp::Reported("2026-01-01T00:00:00.000Z".to_owned())
    );
}

#[test]
fn an_empty_string_description_projects_as_an_empty_line() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(issue_body("ENG-1", "2026-01-01T00:00:00.000Z", "\"\"")),
    );
    let client = client_for(&server, brief());

    let issue = client.show(&id("ENG-1")).expect("show succeeds");

    assert_eq!(
        issue.body, "A title\n",
        "Linear's empty description projects as an empty line, where Jira's \
         absent one projects as the literal null"
    );
}

#[test]
fn a_null_or_absent_stamp_is_not_reported() {
    for description in ["null", "\"\""] {
        let server = MockServer::start();
        server.route(
            RequestKey::post(GRAPHQL),
            json_route(format!(
                "{{\"data\":{{\"issue\":{{\"identifier\":\"ENG-1\",\
                 \"title\":\"t\",\"updatedAt\":null,\
                 \"description\":{description}}}}}}}"
            )),
        );
        let client = client_for(&server, brief());

        let issue = client.show(&id("ENG-1")).expect("show succeeds");

        assert_eq!(issue.updated, RemoteTimestamp::NotReported);
    }
}

#[test]
fn an_empty_request_makes_no_remote_call() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(500));
    let client = client_for(&server, brief());

    let outcome = client.fetch_all(&[]).expect("an empty request succeeds");

    assert_eq!(server.hits(&key), 0);
    assert!(outcome.found.is_empty());
    assert!(outcome.absent.is_empty());
    assert!(outcome.indeterminate.is_empty());
}

#[test]
fn a_flat_filter_bag_groups_same_key_values_into_one_in_clause() {
    let server = MockServer::start();
    serve_issues(&server);
    let client = client_with_resolvers(
        &server,
        resolvers(&json!({
            "baseTeam": TEAM_ID,
            "labels": [],
            "teams": [complete_entry(TEAM_ID, TEAM_KEY, &json!({
                "states": [state("open-uuid", "open")],
                "labels": [label("a-uuid", "a"), label("b-uuid", "b")]
            }))]
        })),
    );

    client
        .search(&scope_over(
            &[TEAM_ID],
            &[("label", "a"), ("label", "b"), ("state", "open")],
        ))
        .expect("search succeeds");

    let filter = sent_filter(&server);
    assert_eq!(
        filter["labels"]["id"]["in"],
        json!(["a-uuid", "b-uuid"]),
        "same-key values OR into one `in`: {filter}"
    );
    assert_eq!(
        filter["state"]["id"]["eq"], "open-uuid",
        "a single state keeps its resolved `eq` form: {filter}"
    );
}

#[test]
fn duplicate_ids_are_deduplicated() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(search_body(&["ENG-1"], None)),
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-1")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
}

#[test]
fn every_search_request_carries_an_explicit_first() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), json_route(search_body(&["ENG-1"], None)));
    let client = client_for(&server, brief());

    client
        .fetch_all(&[id("ENG-1")])
        .expect("fetch_all succeeds");

    let sent: Value =
        serde_json::from_slice(&server.last_body(&key).expect("a body"))
            .expect("JSON");
    assert_eq!(
        sent["variables"]["first"], 250,
        "the API's default of 50 multiplied by complexity is what the \
         explicit value exists to bound"
    );
    assert_eq!(sent["variables"]["filter"]["team"]["id"]["eq"], TEAM_ID);
}

#[test]
fn an_unfound_in_team_id_is_absent_when_the_retrieval_completed() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(search_body(&["ENG-1"], None)),
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
fn an_id_outside_the_configured_team_is_indeterminate_not_absent() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(search_body(&["ENG-1"], None)),
    );
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("OPS-7")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
    assert!(
        outcome.absent.is_empty(),
        "the search never had scope to see OPS-7, so its absence is unproven"
    );
    assert_eq!(outcome.indeterminate, vec![id("OPS-7")]);
    assert_eq!(TEAM_KEY, "ENG");
}

#[test]
fn without_a_known_team_key_no_absence_can_be_proved() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(search_body(&["ENG-1"], None)),
    );
    // Only linear.team_id configured: the UUID alone cannot answer whether an
    // identifier was ever in scope.
    let client = client_with(&server.base_url(), brief(), None);

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-2")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.found.len(), 1);
    assert!(outcome.absent.is_empty());
    assert_eq!(outcome.indeterminate, vec![id("ENG-2")]);
}

#[test]
fn a_failed_search_reports_every_unfound_id_indeterminate() {
    let server = MockServer::start();
    server.route(RequestKey::post(GRAPHQL), Route::Status(500));
    let client = client_for(&server, brief());

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("ENG-2")])
        .expect("a transport-level failure is an Ok with the partition");

    assert!(outcome.absent.is_empty());
    assert_eq!(outcome.indeterminate.len(), 2);
}

#[test]
fn a_stamp_absent_from_a_bulk_row_is_still_found() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(
            "{\"data\":{\"issues\":{\"nodes\":[{\"identifier\":\"ENG-1\"}],\
             \"pageInfo\":{\"hasNextPage\":false}}}}"
                .to_owned(),
        ),
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
fn an_unsafe_identifier_is_a_preflight_error() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(key.clone(), Route::Status(200));
    let client = client_for(&server, brief());

    let error = client
        .fetch_all(&[id("ENG-1"), id("bad\nkey")])
        .expect_err("an unsafe id fails before any request");

    assert!(matches!(error, TrackerError::Retryable { .. }), "{error}");
    assert_eq!(server.hits(&key), 0);
}

#[test]
fn a_404_shaped_read_failure_is_retryable_never_terminal() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Json {
            status: 400,
            body: "{\"errors\":[{\"message\":\"Entity not found\"}]}"
                .to_owned(),
        },
    );
    let client = client_for(&server, brief());

    let error = client.show(&id("ENG-404")).expect_err("the read fails");

    assert!(matches!(error, TrackerError::Retryable { .. }), "{error}");
}

#[test]
fn the_keyed_read_uses_its_own_cap_not_the_discovery_cap() {
    // The keyed read and discovery share `page_all`, so each must be handed its
    // own cap. A low discovery cap must not truncate the keyed read: fetch_all
    // pages the three-page walk to completion and finds the id.
    let found = MockServer::start();
    found.route(RequestKey::post(GRAPHQL), three_pages());
    let outcome =
        client_for(&found, caps(Ceiling::Bounded(1), Ceiling::Bounded(50)))
            .fetch_all(&[id("ENG-3")])
            .expect("fetch_all succeeds");
    assert_eq!(
        outcome.found.len(),
        1,
        "the keyed read ignored the low discovery cap"
    );

    // A low keyed-read cap truncates it: the unseen id is indeterminate, never
    // absent, even with a generous discovery cap.
    let capped = MockServer::start();
    capped.route(RequestKey::post(GRAPHQL), three_pages());
    let outcome =
        client_for(&capped, caps(Ceiling::Bounded(50), Ceiling::Bounded(2)))
            .fetch_all(&[id("ENG-3")])
            .expect("a cut-short keyed read is an Ok with the partition");
    assert!(
        outcome.absent.is_empty(),
        "a truncated read must not infer absence"
    );
    assert_eq!(outcome.indeterminate, vec![id("ENG-3")]);
}

/// One `teams` enumeration page: the given `(id, name, key)` nodes and, when
/// `next` is set, a cursor to the following page.
fn teams_body(teams: &[(&str, &str, &str)], next: Option<&str>) -> String {
    let nodes: Vec<String> = teams
        .iter()
        .map(|(id, name, key)| {
            format!("{{\"id\":\"{id}\",\"name\":\"{name}\",\"key\":\"{key}\"}}")
        })
        .collect();
    let page = next.map_or_else(
        || "{\"hasNextPage\":false,\"endCursor\":null}".to_owned(),
        |cursor| format!("{{\"hasNextPage\":true,\"endCursor\":\"{cursor}\"}}"),
    );
    format!(
        "{{\"data\":{{\"teams\":{{\"nodes\":[{}],\"pageInfo\":{page}}}}}}}",
        nodes.join(",")
    )
}

#[test]
fn an_additional_team_item_reconciles_rather_than_sticking_indeterminate() {
    let server = MockServer::start();
    // The base-team search returns ENG-1; the additional-team search returns
    // OPS-7. A base-only reconcile read would leave OPS-7 indeterminate
    // forever.
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Sequence(vec![
            json_route(search_body(&["ENG-1"], None)),
            json_route(search_body(&["OPS-7"], None)),
        ]),
    );
    let client = client_with_teams(
        &server,
        brief(),
        &[(TEAM_KEY, TEAM_ID), ("OPS", "ops-uuid")],
    );

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("OPS-7")])
        .expect("fetch_all succeeds");

    let found: Vec<&str> =
        outcome.found.iter().map(|(id, _)| id.as_str()).collect();
    assert!(found.contains(&"ENG-1"), "the base-team item reconciles");
    assert!(
        found.contains(&"OPS-7"),
        "the additional-team item reconciles from its own team's page"
    );
    assert!(
        outcome.indeterminate.is_empty(),
        "an item from a catalogued additional team is no longer indeterminate"
    );
}

#[test]
fn an_additional_team_item_is_provably_absent_when_its_team_read_completes() {
    let server = MockServer::start();
    // Both team reads complete, and neither returns OPS-7, so — because OPS is
    // catalogued and thus in scope — OPS-7 is provably absent, not
    // indeterminate.
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Sequence(vec![
            json_route(search_body(&["ENG-1"], None)),
            json_route(search_body(&[], None)),
        ]),
    );
    let client = client_with_teams(
        &server,
        brief(),
        &[(TEAM_KEY, TEAM_ID), ("OPS", "ops-uuid")],
    );

    let outcome = client
        .fetch_all(&[id("ENG-1"), id("OPS-7")])
        .expect("fetch_all succeeds");

    assert_eq!(outcome.absent, vec![id("OPS-7")]);
    assert!(outcome.indeterminate.is_empty());
}

#[test]
fn fetch_all_pages_the_base_team_once_when_it_is_catalogued() {
    let server = MockServer::start();
    let key = RequestKey::post(GRAPHQL);
    server.route(
        key.clone(),
        Route::Sequence(vec![
            json_route(search_body(&["ENG-1"], None)),
            json_route(search_body(&[], None)),
        ]),
    );
    let catalogue = json!({
        "baseTeam": TEAM_ID,
        "teams": [
            { "id": TEAM_ID, "key": TEAM_KEY, "name": "Engineering" },
            { "id": "ops-uuid", "key": "OPS", "name": "Operations" }
        ]
    });
    let client = client_with_resolvers(
        &server,
        Catalogue::from_text(&catalogue.to_string()).resolver_set(),
    );

    client
        .fetch_all(&[id("ENG-1")])
        .expect("fetch_all succeeds");

    assert_eq!(
        server.hits(&key),
        2,
        "the base team and the one additional team, each paged once"
    );
}

#[test]
fn enumerate_visible_entities_paginates_to_exhaustion() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Sequence(vec![
            json_route(teams_body(
                &[("eng-uuid", "Engineering", "ENG")],
                Some("p2"),
            )),
            json_route(teams_body(&[("ops-uuid", "Operations", "OPS")], None)),
        ]),
    );
    let client = client_for(&server, brief());

    let visible = client
        .enumerate_visible_entities()
        .expect("enumeration succeeds");

    let keys: Vec<&str> =
        visible.iter().map(|entity| entity.key.as_str()).collect();
    assert_eq!(
        keys,
        vec!["ENG", "OPS"],
        "a visible-team set spanning two pages is fully enumerated"
    );
    assert_eq!(
        visible[1].identifier, "ops-uuid",
        "each entity carries its search identifier (the team UUID)"
    );
}

#[test]
fn a_failed_enumeration_page_fails_loud_rather_than_returning_a_subset() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        Route::Sequence(vec![
            json_route(teams_body(
                &[("eng-uuid", "Engineering", "ENG")],
                Some("p2"),
            )),
            Route::Status(500),
        ]),
    );
    let client = client_for(&server, brief());

    let error = client
        .enumerate_visible_entities()
        .expect_err("a failed enumeration page is an error, not a subset");

    assert!(matches!(error, TrackerError::Retryable { .. }));
}

const ISSUES: &str = "issues";
const IDENTITIES: &str = "TeamIdentities";
const STATES: &str = "TeamStates";
const LABELS: &str = "TeamLabels";
const WORKSPACE_LABELS: &str = "WorkspaceLabels";
const MEMBERS: &str = "TeamMembers";
const PROJECTS: &str = "TeamProjects";
const SECTION_OPERATIONS: [&str; 6] = [
    IDENTITIES,
    STATES,
    LABELS,
    WORKSPACE_LABELS,
    MEMBERS,
    PROJECTS,
];

const OPS_ID: &str = "ops-uuid";

fn serve_issues(server: &MockServer) {
    server.route(
        RequestKey::graphql(ISSUES),
        json_route(search_body(&[], None)),
    );
}

fn sent_filter(server: &MockServer) -> Value {
    let sent: Value = serde_json::from_slice(
        &server
            .last_body(&RequestKey::graphql(ISSUES))
            .expect("an issues request"),
    )
    .expect("JSON");
    sent["variables"]["filter"].clone()
}

/// A resolved scope over `teams`, the first as the base, carrying `filters`
/// as pre-flight passes them on.
fn scope_over(teams: &[&str], filters: &[(&str, &str)]) -> SearchScope {
    SearchScope {
        entities: tracker::EntityScope::Keyed {
            base: teams.first().map(|id| (*id).to_owned()),
            additional: teams
                .iter()
                .skip(1)
                .map(|id| (*id).to_owned())
                .collect(),
        },
        filters: filters
            .iter()
            .map(|(family, value)| {
                (format!("validated:{family}"), (*value).to_owned())
            })
            .collect(),
    }
}

fn connection(root: &str, nodes: &Value, next: Option<&str>) -> Route {
    json_route(
        json!({
            "data": {
                root: {
                    "nodes": nodes,
                    "pageInfo": {
                        "hasNextPage": next.is_some(),
                        "endCursor": next,
                    },
                },
            },
        })
        .to_string(),
    )
}

fn serve_identities(server: &MockServer, teams: &[(&str, &str)]) {
    let nodes: Vec<Value> = teams
        .iter()
        .map(|(id, key)| json!({ "id": id, "key": key, "name": key }))
        .collect();
    server.route(
        RequestKey::graphql(IDENTITIES),
        connection("teams", &Value::Array(nodes), None),
    );
}

fn serve_states(server: &MockServer, states: &[(&str, &str, &str)]) {
    let nodes: Vec<Value> = states
        .iter()
        .map(|(id, name, team)| {
            json!({ "id": id, "name": name, "type": "started",
                    "position": 1, "archivedAt": null,
                    "team": { "id": team } })
        })
        .collect();
    server.route(
        RequestKey::graphql(STATES),
        connection("workflowStates", &Value::Array(nodes), None),
    );
}

fn section_hits(server: &MockServer) -> Vec<(&'static str, usize)> {
    SECTION_OPERATIONS
        .iter()
        .map(|operation| {
            (*operation, server.hits(&RequestKey::graphql(operation)))
        })
        .filter(|(_, hits)| *hits > 0)
        .collect()
}

#[derive(Default)]
struct RecordingBackfill(Mutex<Vec<LiveCatalogueData>>);

impl RecordingBackfill {
    fn held(&self) -> Vec<LiveCatalogueData> {
        self.0.lock().expect("unpoisoned").clone()
    }
}

impl CatalogueBackfill for RecordingBackfill {
    fn hold(&self, live: LiveCatalogueData) {
        self.0.lock().expect("unpoisoned").push(live);
    }
}

fn eng_with_state() -> Value {
    complete_entry(
        TEAM_ID,
        TEAM_KEY,
        &json!({
            "states": [state("s-eng-ip", "In Progress")]
        }),
    )
}

fn catalogue_with(teams: &[Value]) -> ResolverSet {
    resolvers(&json!({ "baseTeam": TEAM_ID, "labels": [], "teams": teams }))
}

#[test]
fn search_completes_state_for_each_covered_scoped_team() {
    let server = MockServer::start();
    serve_issues(&server);
    let client = client_with_resolvers(
        &server,
        catalogue_with(&[
            eng_with_state(),
            complete_entry(
                OPS_ID,
                "OPS",
                &json!({
                    "states": [state("s-ops-ip", "In Progress")]
                }),
            ),
        ]),
    );

    client
        .search(&scope_over(&[TEAM_ID, OPS_ID], &[("state", "In Progress")]))
        .expect("search succeeds");

    assert_eq!(
        sent_filter(&server)["state"]["id"]["in"],
        json!(["s-eng-ip", "s-ops-ip"])
    );
    assert_eq!(section_hits(&server), Vec::new(), "nothing is fetched");
}

#[test]
fn search_fetches_an_uncovered_team_and_includes_its_ids() {
    let server = MockServer::start();
    serve_issues(&server);
    serve_identities(&server, &[(OPS_ID, "OPS")]);
    serve_states(&server, &[("s-ops-ip", "In Progress", OPS_ID)]);
    let client =
        client_with_resolvers(&server, catalogue_with(&[eng_with_state()]));

    client
        .search(&scope_over(&[OPS_ID], &[("state", "In Progress")]))
        .expect("search succeeds");

    assert_eq!(sent_filter(&server)["state"]["id"]["eq"], "s-ops-ip");
}

#[test]
fn search_fetches_an_incomplete_synced_team_and_includes_its_ids() {
    let server = MockServer::start();
    serve_issues(&server);
    serve_identities(&server, &[(OPS_ID, "OPS")]);
    serve_states(&server, &[("s-ops-ip", "In Progress", OPS_ID)]);
    let client = client_with_resolvers(
        &server,
        catalogue_with(&[eng_with_state(), entry(OPS_ID, "OPS", &json!({}))]),
    );

    client
        .search(&scope_over(&[OPS_ID], &[("state", "In Progress")]))
        .expect("search succeeds");

    assert_eq!(sent_filter(&server)["state"]["id"]["eq"], "s-ops-ip");
}

#[test]
fn covered_and_uncovered_teams_both_contribute() {
    let server = MockServer::start();
    serve_issues(&server);
    serve_identities(&server, &[(OPS_ID, "OPS")]);
    serve_states(&server, &[("s-ops-ip", "In Progress", OPS_ID)]);
    let client =
        client_with_resolvers(&server, catalogue_with(&[eng_with_state()]));

    client
        .search(&scope_over(&[TEAM_ID, OPS_ID], &[("state", "In Progress")]))
        .expect("search succeeds");

    assert_eq!(
        sent_filter(&server)["state"]["id"]["in"],
        json!(["s-eng-ip", "s-ops-ip"])
    );
    let identities: Value = serde_json::from_slice(
        &server
            .last_body(&RequestKey::graphql(IDENTITIES))
            .expect("an identity lookup"),
    )
    .expect("JSON");
    assert_eq!(
        identities["variables"]["ids"],
        json!([OPS_ID]),
        "only the uncovered team is fetched"
    );
}

#[test]
fn search_fetches_only_the_sections_the_configured_families_need() {
    let server = MockServer::start();
    serve_issues(&server);
    serve_identities(&server, &[(OPS_ID, "OPS")]);
    serve_states(&server, &[("s-ops-ip", "In Progress", OPS_ID)]);
    let client =
        client_with_resolvers(&server, catalogue_with(&[eng_with_state()]));

    client
        .search(&scope_over(&[OPS_ID], &[("state", "In Progress")]))
        .expect("search succeeds");

    assert_eq!(section_hits(&server), vec![(IDENTITIES, 1), (STATES, 1)]);
}

#[test]
fn a_legacy_base_entry_filtered_on_state_needs_no_fetch() {
    let server = MockServer::start();
    serve_issues(&server);
    let client = client_with_resolvers(
        &server,
        resolvers(&json!({
            "team": { "id": TEAM_ID, "key": TEAM_KEY, "name": "Engineering" },
            "workflowStates": [
                { "id": "s-todo", "name": "Todo", "type": "unstarted",
                  "position": 0 }
            ]
        })),
    );

    client
        .search(&scope_over(&[TEAM_ID], &[("state", "Todo")]))
        .expect("search succeeds");

    assert_eq!(sent_filter(&server)["state"]["id"]["eq"], "s-todo");
    assert_eq!(section_hits(&server), Vec::new());
}

#[test]
fn nothing_is_fetched_when_every_scoped_team_is_covered() {
    let server = MockServer::start();
    serve_issues(&server);
    let backfill = Arc::new(RecordingBackfill::default());
    let client = client_holding_into(
        &server,
        catalogue_with(&[eng_with_state()]),
        backfill.clone(),
    );

    client
        .search(&scope_over(&[TEAM_ID], &[("state", "In Progress")]))
        .expect("search succeeds");

    assert_eq!(section_hits(&server), Vec::new());
    assert!(backfill.held().is_empty(), "nothing fetched, nothing held");
}

#[test]
fn nothing_is_fetched_without_filters() {
    let server = MockServer::start();
    serve_issues(&server);
    let client = client_with_resolvers(&server, catalogue_with(&[]));

    client
        .search(&SearchScope {
            filters: vec![("text".to_owned(), "needle".to_owned())],
            ..scope_over(&[OPS_ID], &[])
        })
        .expect("search succeeds");

    assert_eq!(section_hits(&server), Vec::new());
    assert_eq!(
        sent_filter(&server)["title"]["containsIgnoreCase"],
        "needle"
    );
}

#[test]
fn search_refuses_a_family_left_with_no_ids_without_paging() {
    let server = MockServer::start();
    let client =
        client_with_resolvers(&server, catalogue_with(&[eng_with_state()]));

    let error = client
        .search(&scope_over(&[TEAM_ID], &[("state", "Nope")]))
        .expect_err("no scoped team carries the state");

    let TrackerError::Unconfigured { detail } = error else {
        panic!("a filter refusal is a configuration fault: {error:?}");
    };
    assert!(detail.starts_with("pull filters could not be resolved:"));
    assert!(detail.contains("E_SEARCH_UNKNOWN_STATE"), "{detail}");
    assert_eq!(server.hits(&RequestKey::graphql(ISSUES)), 0);
}

#[test]
fn fetched_entries_are_held_exactly_once() {
    let server = MockServer::start();
    serve_issues(&server);
    serve_identities(&server, &[(OPS_ID, "OPS")]);
    serve_states(&server, &[("s-ops-ip", "In Progress", OPS_ID)]);
    let backfill = Arc::new(RecordingBackfill::default());
    let client = client_holding_into(
        &server,
        catalogue_with(&[eng_with_state()]),
        backfill.clone(),
    );

    client
        .search(&scope_over(&[TEAM_ID, OPS_ID], &[("state", "In Progress")]))
        .expect("search succeeds");

    let held = backfill.held();
    assert_eq!(held.len(), 1, "one hold per search");
    let ids: Vec<&str> = held[0]
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(ids, vec![OPS_ID], "only the fetched team is held");
    assert!(held[0].entries[0].states.is_some());
}

#[test]
fn a_later_section_failure_holds_nothing() {
    let server = MockServer::start();
    serve_issues(&server);
    serve_identities(&server, &[(OPS_ID, "OPS")]);
    serve_states(&server, &[("s-ops-ip", "In Progress", OPS_ID)]);
    server.route(
        RequestKey::graphql(LABELS),
        Route::Json {
            status: 500,
            body: "{}".to_owned(),
        },
    );
    let backfill = Arc::new(RecordingBackfill::default());
    let client = client_holding_into(
        &server,
        catalogue_with(&[eng_with_state()]),
        backfill.clone(),
    );

    let error = client
        .search(&scope_over(
            &[OPS_ID],
            &[("state", "In Progress"), ("label", "Bug")],
        ))
        .expect_err("the labels pass failed");

    assert!(matches!(error, TrackerError::Retryable { .. }), "{error:?}");
    assert!(backfill.held().is_empty());
    assert_eq!(server.hits(&RequestKey::graphql(ISSUES)), 0);
}

#[test]
fn a_truncated_team_section_fetch_refuses_as_unconfigured() {
    let server = MockServer::start();
    serve_identities(&server, &[(OPS_ID, "OPS")]);
    server.route(
        RequestKey::graphql(STATES),
        connection("workflowStates", &json!([]), Some("more")),
    );
    let client =
        client_with_resolvers(&server, catalogue_with(&[eng_with_state()]));

    let error = client
        .search(&scope_over(&[OPS_ID], &[("state", "In Progress")]))
        .expect_err("the states connection runs past its ceiling");

    let TrackerError::Unconfigured { detail } = error else {
        panic!("truncation is a configuration fault: {error:?}");
    };
    assert!(detail.contains("workflowStates"), "{detail}");
    assert!(detail.contains("60"), "names the ceiling: {detail}");
    assert!(detail.contains("narrow"), "{detail}");
}

#[test]
fn search_refuses_an_unresolved_scope() {
    let server = MockServer::start();
    let client =
        client_with_resolvers(&server, catalogue_with(&[eng_with_state()]));

    let error = client
        .search(&SearchScope {
            filters: vec![("state".to_owned(), "In Progress".to_owned())],
            ..scope_over(&[TEAM_ID], &[])
        })
        .expect_err("a named pair never passed pre-flight");

    let TrackerError::Unconfigured { detail } = error else {
        panic!("an unresolved scope is a configuration fault: {error:?}");
    };
    assert!(detail.contains("E_SEARCH_UNRESOLVED_SCOPE"), "{detail}");
    assert_eq!(server.hits(&RequestKey::post(GRAPHQL)), 0);
}

#[test]
fn a_label_filter_fetches_workspace_labels_only_when_the_catalogue_lacks_them()
{
    for (workspace_labels, expected_fetches) in
        [(None, 1), (Some(json!([])), 0)]
    {
        let server = MockServer::start();
        serve_issues(&server);
        server.route(
            RequestKey::graphql(WORKSPACE_LABELS),
            connection("issueLabels", &json!([]), None),
        );
        let mut catalogue = json!({
            "baseTeam": TEAM_ID,
            "teams": [complete_entry(TEAM_ID, TEAM_KEY, &json!({
                "labels": [label("l-bug", "Bug")]
            }))]
        });
        if let Some(labels) = workspace_labels {
            catalogue["labels"] = labels;
        }
        let client = client_with_resolvers(&server, resolvers(&catalogue));

        client
            .search(&scope_over(&[TEAM_ID], &[("label", "Bug")]))
            .expect("search succeeds");

        assert_eq!(
            section_hits(&server),
            if expected_fetches == 0 {
                Vec::new()
            } else {
                vec![(WORKSPACE_LABELS, expected_fetches)]
            }
        );
        assert_eq!(sent_filter(&server)["labels"]["id"]["eq"], "l-bug");
    }
}
