//! Init discovery: each query's document, and the cache shape it produces
//! against a committed golden — including the team-entry fetch init,
//! completion and healing share.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use linear_client::catalogue::{CatalogueSection, SectionSet};
use linear_client::discovery::{SectionFetch, TeamEntryFetch};
use linear_client::SurfaceError;
use serde_json::{json, Value};
use support::client::client_for;
use tracker_support::TransportConfig;

const GRAPHQL: &str = "/graphql";

fn json_route(body: &str) -> Route {
    Route::Json {
        status: 200,
        body: body.to_owned(),
    }
}

fn sent_document(server: &MockServer) -> String {
    let body = server
        .last_body(&RequestKey::post(GRAPHQL))
        .expect("a request body");
    let value: Value = serde_json::from_slice(&body).expect("JSON");
    value["query"].as_str().expect("a document").to_owned()
}

fn golden(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path).expect("the golden is readable");
    serde_json::from_str(&text).expect("the golden is JSON")
}

#[test]
fn discover_viewer_queries_the_viewer_and_matches_the_golden() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(
            "{\"data\":{\"viewer\":{\"id\":\"user-1\",\
             \"name\":\"Ada Lovelace\"}}}",
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    let shape = client.discover_viewer().expect("discovery succeeds");

    assert!(sent_document(&server).contains("viewer"));
    assert_eq!(shape, golden("viewer.golden.json"));
}

#[test]
fn discover_viewer_without_an_id_is_a_bad_response() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route("{\"data\":{\"viewer\":{\"name\":\"Ada\"}}}"),
    );
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .discover_viewer()
        .expect_err("no viewer id is a failure");

    assert!(matches!(error, SurfaceError::BadResponse { .. }), "{error}");
}

#[test]
fn list_teams_queries_teams_and_returns_the_nodes() {
    let server = MockServer::start();
    server.route(
        RequestKey::post(GRAPHQL),
        json_route(
            "{\"data\":{\"teams\":{\"nodes\":[\
             {\"id\":\"team-1\",\"name\":\"Engineering\",\"key\":\"ENG\"}]}}}",
        ),
    );
    let client = client_for(&server, TransportConfig::default());

    let teams = client.list_teams().expect("listing succeeds");

    assert!(sent_document(&server).contains("teams"));
    let teams = teams.as_array().expect("an array");
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0]["key"], "ENG");
}

#[test]
fn team_enumeration_stays_unbounded() {
    let server = MockServer::start();
    let pages: Vec<Route> = (1..=70)
        .map(|index| {
            json_route(&format!(
                "{{\"data\":{{\"teams\":{{\"nodes\":[{{\"id\":\"team-{index}\",\
                 \"name\":\"Team {index}\",\"key\":\"T{index}\"}}],\
                 \"pageInfo\":{{\"hasNextPage\":{},\
                 \"endCursor\":\"c{index}\"}}}}}}}}",
                index < 70
            ))
        })
        .collect();
    server.route(RequestKey::post(GRAPHQL), Route::Sequence(pages));
    let client = client_for(&server, TransportConfig::default());

    let teams = client.list_teams().expect("listing succeeds");

    assert_eq!(teams.as_array().map(Vec::len), Some(70));
}

const IDENTITIES: &str = "TeamIdentities";
const STATES: &str = "TeamStates";
const LABELS: &str = "TeamLabels";
const WORKSPACE_LABELS: &str = "WorkspaceLabels";
const MEMBERS: &str = "TeamMembers";
const PROJECTS: &str = "TeamProjects";

fn page(root: &str, nodes: &Value, next: Option<&str>) -> Route {
    json_route(
        &json!({
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

fn identity(id: &str) -> Value {
    json!({ "id": id, "key": id.to_uppercase(), "name": format!("Team {id}") })
}

fn serve_identities(server: &MockServer, ids: &[&str]) {
    let nodes: Vec<Value> = ids.iter().map(|id| identity(id)).collect();
    server.route(
        RequestKey::graphql(IDENTITIES),
        page("teams", &Value::Array(nodes), None),
    );
}

fn state(id: &str, team: &str) -> Value {
    json!({
        "id": id, "name": format!("State {id}"), "type": "started",
        "position": 1, "archivedAt": null, "team": { "id": team },
    })
}

fn label(id: &str, team: &str) -> Value {
    json!({
        "id": id, "name": format!("Label {id}"), "archivedAt": null,
        "team": { "id": team },
    })
}

fn nested_teams(teams: &[&str], more: bool) -> Value {
    let nodes: Vec<Value> =
        teams.iter().map(|id| json!({ "id": id })).collect();
    json!({ "nodes": nodes, "pageInfo": { "hasNextPage": more } })
}

fn member(id: &str, teams: &[&str]) -> Value {
    json!({
        "id": id, "name": format!("user-{id}"), "displayName": id,
        "email": format!("{id}@example.com"), "active": true,
        "teams": nested_teams(teams, false),
    })
}

fn project(id: &str, teams: &[&str]) -> Value {
    json!({
        "id": id, "name": format!("Project {id}"), "archivedAt": null,
        "teams": nested_teams(teams, false),
    })
}

fn only(sections: &[CatalogueSection]) -> SectionSet {
    SectionSet::of(sections)
}

fn fetch(
    server: &MockServer,
    ids: &[&str],
    sections: &SectionSet,
) -> Result<SectionFetch, SurfaceError> {
    let ids: Vec<String> = ids.iter().map(|id| (*id).to_owned()).collect();
    client_for(server, TransportConfig::default())
        .fetch_team_entries(&ids, sections)
}

fn sent_queries(server: &MockServer, operation: &str) -> Vec<Value> {
    server
        .bodies(&RequestKey::graphql(operation))
        .iter()
        .map(|body| serde_json::from_slice(body).expect("JSON"))
        .collect()
}

struct SectionCase {
    section: CatalogueSection,
    operation: &'static str,
    root: &'static str,
    golden_key: &'static str,
    pages: [Value; 2],
    query_mentions: &'static [&'static str],
}

fn section_cases() -> Vec<SectionCase> {
    vec![
        SectionCase {
            section: CatalogueSection::States,
            operation: STATES,
            root: "workflowStates",
            golden_key: "states",
            pages: [json!([state("s-1", "t-a")]), json!([state("s-2", "t-a")])],
            query_mentions: &["workflowStates", "team: { id: { in: $ids } }"],
        },
        SectionCase {
            section: CatalogueSection::Labels,
            operation: LABELS,
            root: "issueLabels",
            golden_key: "labels",
            pages: [json!([label("l-1", "t-a")]), json!([label("l-2", "t-a")])],
            query_mentions: &["issueLabels", "team: { id: { in: $ids } }"],
        },
        SectionCase {
            section: CatalogueSection::Members,
            operation: MEMBERS,
            root: "users",
            golden_key: "members",
            pages: [
                json!([member("m-1", &["t-a"])]),
                json!([member("m-2", &["t-a"])]),
            ],
            query_mentions: &["users(", "teams(first: 10)"],
        },
        SectionCase {
            section: CatalogueSection::Projects,
            operation: PROJECTS,
            root: "projects",
            golden_key: "projects",
            pages: [
                json!([project("p-1", &["t-a"])]),
                json!([project("p-2", &["t-a"])]),
            ],
            query_mentions: &[
                "projects(",
                "accessibleTeams: { some: { id: { in: $ids } } }",
                "teams(first: 10)",
            ],
        },
    ]
}

#[test]
fn fetching_team_entries_pages_each_section_to_exhaustion() {
    let golden = golden("team-entries.golden.json");
    for case in section_cases() {
        let server = MockServer::start();
        serve_identities(&server, &["t-a"]);
        let [first, second] = &case.pages;
        server.route(
            RequestKey::graphql(case.operation),
            Route::Sequence(vec![
                page(case.root, first, Some("cursor-1")),
                page(case.root, second, None),
            ]),
        );

        let fetched = fetch(&server, &["t-a"], &only(&[case.section]))
            .unwrap_or_else(|error| panic!("{}: {error}", case.golden_key));

        let sent = sent_queries(&server, case.operation);
        assert_eq!(sent.len(), 2, "{}", case.golden_key);
        let document = sent[0]["query"].as_str().expect("a document");
        for mention in case.query_mentions {
            assert!(
                document.contains(mention),
                "{}: {document}",
                case.golden_key
            );
        }
        let entries = serde_json::to_value(&fetched.entries).expect("JSON");
        assert_eq!(entries, golden[case.golden_key], "{}", case.golden_key);
    }
}

#[test]
fn each_section_query_sends_its_page_sizes() {
    let expected = [
        (STATES, "workflowStates(first: 250"),
        (LABELS, "issueLabels(first: 250"),
        (MEMBERS, "users(first: 100"),
        (PROJECTS, "projects(first: 100"),
        (WORKSPACE_LABELS, "issueLabels(first: 250"),
        (IDENTITIES, "teams(first: 250"),
    ];
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(STATES),
        page("workflowStates", &json!([state("s-1", "t-a")]), None),
    );
    server.route(
        RequestKey::graphql(LABELS),
        page("issueLabels", &json!([]), None),
    );
    server.route(
        RequestKey::graphql(WORKSPACE_LABELS),
        page("issueLabels", &json!([]), None),
    );
    server.route(
        RequestKey::graphql(MEMBERS),
        page("users", &json!([]), None),
    );
    server.route(
        RequestKey::graphql(PROJECTS),
        page("projects", &json!([]), None),
    );

    fetch(&server, &["t-a"], &SectionSet::all()).expect("the fetch succeeds");

    for (operation, prefix) in expected {
        let sent = sent_queries(&server, operation);
        let document = sent[0]["query"].as_str().expect("a document");
        assert!(document.contains(prefix), "{operation}: {document}");
    }
}

#[test]
fn fetching_team_entries_filters_by_the_requested_team_ids() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a", "t-b"]);
    server.route(
        RequestKey::graphql(STATES),
        page(
            "workflowStates",
            &json!([state("s-1", "t-a"), state("s-2", "t-b")]),
            None,
        ),
    );
    server.route(
        RequestKey::graphql(PROJECTS),
        page("projects", &json!([]), None),
    );

    fetch(
        &server,
        &["t-a", "t-b"],
        &only(&[CatalogueSection::States, CatalogueSection::Projects]),
    )
    .expect("the fetch succeeds");

    for operation in [IDENTITIES, STATES, PROJECTS] {
        let sent = sent_queries(&server, operation);
        assert_eq!(
            sent[0]["variables"]["ids"],
            json!(["t-a", "t-b"]),
            "{operation}"
        );
    }
}

#[test]
fn fetching_team_entries_requests_archived_and_disabled_entities() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(STATES),
        page("workflowStates", &json!([state("s-1", "t-a")]), None),
    );
    server.route(
        RequestKey::graphql(LABELS),
        page("issueLabels", &json!([]), None),
    );
    server.route(
        RequestKey::graphql(WORKSPACE_LABELS),
        page("issueLabels", &json!([]), None),
    );
    server.route(
        RequestKey::graphql(MEMBERS),
        page("users", &json!([]), None),
    );
    server.route(
        RequestKey::graphql(PROJECTS),
        page("projects", &json!([]), None),
    );

    fetch(&server, &["t-a"], &SectionSet::all()).expect("the fetch succeeds");

    for (operation, flag) in [
        (IDENTITIES, "includeArchived: true"),
        (STATES, "includeArchived: true"),
        (LABELS, "includeArchived: true"),
        (WORKSPACE_LABELS, "includeArchived: true"),
        (PROJECTS, "includeArchived: true"),
        (MEMBERS, "includeDisabled: true"),
    ] {
        let sent = sent_queries(&server, operation);
        let document = sent[0]["query"].as_str().expect("a document");
        assert!(document.contains(flag), "{operation}: {document}");
    }
}

#[test]
fn fetching_only_the_requested_sections() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(STATES),
        page("workflowStates", &json!([state("s-1", "t-a")]), None),
    );
    server.route(
        RequestKey::graphql(LABELS),
        page("issueLabels", &json!([]), None),
    );

    let fetched = fetch(
        &server,
        &["t-a"],
        &only(&[CatalogueSection::States, CatalogueSection::Labels]),
    )
    .expect("the fetch succeeds");

    assert_eq!(server.hits(&RequestKey::graphql(IDENTITIES)), 1);
    assert_eq!(server.hits(&RequestKey::graphql(STATES)), 1);
    assert_eq!(server.hits(&RequestKey::graphql(LABELS)), 1);
    assert_eq!(server.hits(&RequestKey::graphql(MEMBERS)), 0);
    assert_eq!(server.hits(&RequestKey::graphql(PROJECTS)), 0);
    assert_eq!(server.hits(&RequestKey::graphql(WORKSPACE_LABELS)), 0);
    let entry = &fetched.entries[0];
    assert!(entry.members.is_none() && entry.projects.is_none());
    assert!(fetched.workspace_labels.is_none());
}

#[test]
fn fetching_team_entries_threads_the_end_cursor() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(STATES),
        Route::Sequence(vec![
            page(
                "workflowStates",
                &json!([state("s-1", "t-a")]),
                Some("cursor-1"),
            ),
            page("workflowStates", &json!([state("s-2", "t-a")]), None),
        ]),
    );

    fetch(&server, &["t-a"], &only(&[CatalogueSection::States]))
        .expect("the fetch succeeds");

    let sent = sent_queries(&server, STATES);
    assert_eq!(sent[0]["variables"]["after"], Value::Null);
    assert_eq!(sent[1]["variables"]["after"], "cursor-1");
}

#[test]
fn a_page_two_failure_fails_the_whole_fetch() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(LABELS),
        Route::Sequence(vec![
            page(
                "issueLabels",
                &json!([label("l-1", "t-a")]),
                Some("cursor-1"),
            ),
            json_route("{\"errors\":[{\"message\":\"boom\"}]}"),
        ]),
    );

    let error = fetch(&server, &["t-a"], &only(&[CatalogueSection::Labels]))
        .expect_err("a failed page fails the fetch");

    assert!(
        matches!(error, SurfaceError::GraphQlErrors { .. }),
        "{error}"
    );
}

#[test]
fn a_team_returned_with_no_states_is_a_bad_response() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a", "t-b"]);
    server.route(
        RequestKey::graphql(STATES),
        page("workflowStates", &json!([state("s-1", "t-a")]), None),
    );

    let error =
        fetch(&server, &["t-a", "t-b"], &only(&[CatalogueSection::States]))
            .expect_err("a team with no states is refused");

    assert!(matches!(error, SurfaceError::BadResponse { .. }), "{error}");
    assert!(error.to_string().contains("t-b"), "{error}");
}

#[test]
fn a_node_shared_by_two_requested_teams_appears_in_both_entries() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a", "t-b"]);
    server.route(
        RequestKey::graphql(MEMBERS),
        page("users", &json!([member("m-1", &["t-a", "t-b"])]), None),
    );
    server.route(
        RequestKey::graphql(PROJECTS),
        page("projects", &json!([project("p-1", &["t-a", "t-b"])]), None),
    );

    let fetched = fetch(
        &server,
        &["t-a", "t-b"],
        &only(&[CatalogueSection::Members, CatalogueSection::Projects]),
    )
    .expect("the fetch succeeds");

    for entry in &fetched.entries {
        assert_eq!(entry.members.as_ref().expect("members")[0].id, "m-1");
        assert_eq!(entry.projects.as_ref().expect("projects")[0].id, "p-1");
        assert!(entry.members.as_ref().expect("members")[0].extra.is_empty());
    }
}

#[test]
fn a_node_for_an_unrequested_team_is_dropped() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(MEMBERS),
        page(
            "users",
            &json!([member("m-1", &["t-a", "t-z"]), member("m-2", &["t-z"])]),
            None,
        ),
    );

    let fetched = fetch(&server, &["t-a"], &only(&[CatalogueSection::Members]))
        .expect("the fetch succeeds");

    assert_eq!(fetched.entries.len(), 1);
    let members = fetched.entries[0].members.as_ref().expect("members");
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].id, "m-1");
}

#[test]
fn a_nested_team_connection_past_one_page_fails_loud() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    let mut crowded = project("p-1", &["t-a"]);
    crowded["teams"] = nested_teams(&["t-a"], true);
    server.route(
        RequestKey::graphql(PROJECTS),
        page("projects", &json!([crowded]), None),
    );

    let error = fetch(&server, &["t-a"], &only(&[CatalogueSection::Projects]))
        .expect_err("an overflowing nested connection is never truncated");

    assert!(
        matches!(
            error,
            SurfaceError::CatalogueTruncated {
                connection: "projects.teams",
                ..
            }
        ),
        "{error}"
    );
}

#[test]
fn a_requested_team_absent_from_the_response_is_reported_unreturned() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(STATES),
        page("workflowStates", &json!([state("s-1", "t-a")]), None),
    );

    let fetched = fetch(
        &server,
        &["t-a", "t-gone"],
        &only(&[CatalogueSection::States]),
    )
    .expect("the fetch succeeds");

    assert_eq!(fetched.unreturned, vec!["t-gone".to_owned()]);
    assert_eq!(fetched.entries.len(), 1);
    assert_eq!(fetched.entries[0].id, "t-a");
}

#[test]
fn a_present_team_with_no_labels_or_projects_gets_empty_sections() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(LABELS),
        page("issueLabels", &json!([]), None),
    );
    server.route(
        RequestKey::graphql(PROJECTS),
        page("projects", &json!([]), None),
    );

    let fetched = fetch(
        &server,
        &["t-a"],
        &only(&[CatalogueSection::Labels, CatalogueSection::Projects]),
    )
    .expect("the fetch succeeds");

    assert!(fetched.unreturned.is_empty());
    let entry = &fetched.entries[0];
    assert_eq!(entry.labels, Some(Vec::new()));
    assert_eq!(entry.projects, Some(Vec::new()));
    assert_eq!(entry.key, "T-A");
    assert_eq!(entry.name, "Team t-a");
}

#[test]
fn the_identity_lookup_pages_past_one_page_of_ids() {
    let server = MockServer::start();
    let ids: Vec<String> =
        (0..300).map(|index| format!("t-{index:03}")).collect();
    let first: Vec<Value> = ids[..250].iter().map(|id| identity(id)).collect();
    let second: Vec<Value> = ids[250..].iter().map(|id| identity(id)).collect();
    server.route(
        RequestKey::graphql(IDENTITIES),
        Route::Sequence(vec![
            page("teams", &Value::Array(first), Some("cursor-1")),
            page("teams", &Value::Array(second), None),
        ]),
    );
    server.route(
        RequestKey::graphql(LABELS),
        page("issueLabels", &json!([]), None),
    );
    let requested: Vec<&str> = ids.iter().map(String::as_str).collect();

    let fetched =
        fetch(&server, &requested, &only(&[CatalogueSection::Labels]))
            .expect("the fetch succeeds");

    assert_eq!(server.hits(&RequestKey::graphql(IDENTITIES)), 2);
    assert_eq!(fetched.entries.len(), 300);
    assert!(fetched.unreturned.is_empty());
}

#[test]
fn the_identity_lookup_includes_archived_teams() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(LABELS),
        page("issueLabels", &json!([]), None),
    );

    fetch(&server, &["t-a"], &only(&[CatalogueSection::Labels]))
        .expect("the fetch succeeds");

    let sent = sent_queries(&server, IDENTITIES);
    let document = sent[0]["query"].as_str().expect("a document");
    assert!(document.contains("includeArchived: true"), "{document}");
    assert!(document.contains("id: { in: $ids }"), "{document}");
}

#[test]
fn workspace_labels_are_a_section_of_the_same_fetch() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    server.route(
        RequestKey::graphql(WORKSPACE_LABELS),
        page(
            "issueLabels",
            &json!([{ "id": "w-1", "name": "Security", "archivedAt": null }]),
            None,
        ),
    );

    let fetched = fetch(
        &server,
        &["t-a"],
        &only(&[CatalogueSection::WorkspaceLabels]),
    )
    .expect("the fetch succeeds");

    assert_eq!(server.hits(&RequestKey::graphql(WORKSPACE_LABELS)), 1);
    let document = sent_queries(&server, WORKSPACE_LABELS)[0]["query"]
        .as_str()
        .expect("a document")
        .to_owned();
    assert!(document.contains("team: { null: true }"), "{document}");
    let labels = fetched.workspace_labels.expect("workspace labels");
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].name, "Security");
}

#[test]
fn an_expired_fetch_deadline_sends_no_request() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a"]);
    let client = client_for(
        &server,
        TransportConfig {
            deadline: std::time::Duration::ZERO,
            ..TransportConfig::default()
        },
    );

    let error = client
        .fetch_team_entries(&["t-a".to_owned()], &SectionSet::all())
        .expect_err("an expired deadline stops the fetch");

    assert!(
        matches!(error, SurfaceError::DeadlineExpired { .. }),
        "{error}"
    );
    assert_eq!(server.hits(&RequestKey::post(GRAPHQL)), 0);
}

#[test]
fn a_team_returned_but_not_requested_gets_no_entry() {
    let server = MockServer::start();
    serve_identities(&server, &["t-a", "t-extra"]);
    server.route(
        RequestKey::graphql(LABELS),
        page("issueLabels", &json!([]), None),
    );

    let fetched = fetch(&server, &["t-a"], &only(&[CatalogueSection::Labels]))
        .expect("the fetch succeeds");

    let ids: Vec<&str> = fetched
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(ids, vec!["t-a"]);
}
