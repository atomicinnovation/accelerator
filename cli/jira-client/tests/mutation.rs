//! The rich `create` and `update` payloads: every field maps to the Jira REST
//! shape the retiring bash flows sent, across the `fields` (set) and `update`
//! (incremental) channels.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::mutation::{CreateFields, FieldEdit, IssueType, UpdateFields};
use serde_json::{json, Map, Value};
use support::client::{brief, client_for};
use tracker::ExternalId;

const ISSUE: &str = "/rest/api/3/issue";

fn id(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

fn empty() -> Map<String, Value> {
    Map::new()
}

fn sent(server: &MockServer, key: &RequestKey) -> Value {
    serde_json::from_slice(&server.last_body(key).expect("a body"))
        .expect("JSON")
}

#[test]
fn create_maps_every_field_into_the_rest_payload() {
    let server = MockServer::start();
    let key = RequestKey::post(ISSUE);
    server.route(
        key.clone(),
        Route::Json {
            status: 201,
            body: "{\"key\":\"ENG-9\"}".to_owned(),
        },
    );
    let client = client_for(&server, brief());
    let labels = vec!["backend".to_owned(), "urgent".to_owned()];
    let components = vec!["api".to_owned()];
    let mut custom = Map::new();
    custom.insert("customfield_1".to_owned(), json!(8));

    client
        .create_op(&CreateFields {
            summary: "A title",
            body: "A body\n",
            issue_type: IssueType::Id("10002"),
            project: Some("OPS"),
            assignee: Some("acc-1"),
            reporter: Some("acc-2"),
            priority: Some("High"),
            labels: &labels,
            components: &components,
            parent: Some("OPS-1"),
            custom: &custom,
        })
        .expect("create succeeds");

    let body = sent(&server, &key);
    let fields = &body["fields"];
    assert_eq!(fields["project"]["key"], "OPS", "per-call project override");
    assert_eq!(fields["issuetype"]["id"], "10002", "issuetype-id wins");
    assert_eq!(fields["assignee"]["accountId"], "acc-1");
    assert_eq!(fields["reporter"]["accountId"], "acc-2");
    assert_eq!(fields["priority"]["name"], "High");
    assert_eq!(fields["labels"], json!(["backend", "urgent"]));
    assert_eq!(fields["components"], json!([{"name": "api"}]));
    assert_eq!(fields["parent"]["key"], "OPS-1");
    assert_eq!(fields["customfield_1"], json!(8));
    assert_eq!(fields["description"]["type"], "doc");
}

#[test]
fn create_omits_unset_optional_fields() {
    let server = MockServer::start();
    let key = RequestKey::post(ISSUE);
    server.route(
        key.clone(),
        Route::Json {
            status: 201,
            body: "{\"key\":\"ENG-1\"}".to_owned(),
        },
    );
    let client = client_for(&server, brief());
    let custom = empty();

    client
        .create_op(&CreateFields {
            summary: "S",
            body: "",
            issue_type: IssueType::Default,
            project: None,
            assignee: None,
            reporter: None,
            priority: None,
            labels: &[],
            components: &[],
            parent: None,
            custom: &custom,
        })
        .expect("create succeeds");

    let fields = sent(&server, &key);
    let fields = &fields["fields"];
    assert_eq!(fields["issuetype"]["name"], "Task", "the default type");
    assert!(fields.get("assignee").is_none(), "no empty assignee");
    assert!(fields.get("labels").is_none(), "no empty labels");
    assert!(fields.get("parent").is_none(), "no empty parent");
}

#[test]
fn update_sets_fields_and_incremental_channels_together() {
    let server = MockServer::start();
    let key = RequestKey::put("/rest/api/3/issue/ENG-5");
    server.route(key.clone(), Route::Status(204));
    let client = client_for(&server, brief());
    let add = vec!["needs-review".to_owned()];
    let remove = vec!["stale".to_owned()];
    let custom = empty();

    client
        .update_op(
            &id("ENG-5"),
            &UpdateFields {
                summary: Some("New"),
                body: None,
                priority: Some("Low"),
                assignee: Some(FieldEdit::Clear),
                reporter: None,
                parent: Some(FieldEdit::Clear),
                labels: None,
                add_labels: &add,
                remove_labels: &remove,
                components: None,
                add_components: &[],
                remove_components: &[],
                custom: &custom,
                no_notify: false,
            },
        )
        .expect("update succeeds");

    let body = sent(&server, &key);
    assert_eq!(body["fields"]["summary"], "New");
    assert_eq!(body["fields"]["priority"]["name"], "Low");
    assert_eq!(
        body["fields"]["assignee"]["accountId"],
        Value::Null,
        "Clear unassigns"
    );
    assert_eq!(body["fields"]["parent"], Value::Null, "Clear detaches");
    assert!(
        body["fields"].get("description").is_none(),
        "an unset body is not sent — no summary/description clobber"
    );
    assert_eq!(
        body["update"]["labels"],
        json!([{"add": "needs-review"}, {"remove": "stale"}])
    );
}

#[test]
fn update_replaces_all_labels_through_the_fields_channel() {
    let server = MockServer::start();
    let key = RequestKey::put("/rest/api/3/issue/ENG-5");
    server.route(key.clone(), Route::Status(204));
    let client = client_for(&server, brief());
    let labels = vec!["only".to_owned()];
    let custom = empty();

    client
        .update_op(
            &id("ENG-5"),
            &UpdateFields {
                summary: None,
                body: None,
                priority: None,
                assignee: None,
                reporter: None,
                parent: None,
                labels: Some(&labels),
                add_labels: &[],
                remove_labels: &[],
                components: None,
                add_components: &[],
                remove_components: &[],
                custom: &custom,
                no_notify: true,
            },
        )
        .expect("update succeeds");

    let body = sent(&server, &key);
    assert_eq!(body["fields"]["labels"], json!(["only"]));
    assert!(body.get("update").is_none(), "no incremental channel");
    assert_eq!(
        server.last_query(&key).expect("a query"),
        "notifyUsers=false",
        "--no-notify rides the query string"
    );
}
