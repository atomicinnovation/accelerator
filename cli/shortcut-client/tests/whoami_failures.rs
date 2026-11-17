//! A failed whoami lookup is reported as itself, and is never remembered as
//! an answer.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use serde_json::json;
use serde_json::Value;
use support::client::catalogue_from;
use support::client::catalogue_value;
use support::client::client;
use support::client::client_over;
use support::client::json_route;
use support::client::path;
use support::client::TEAM_KEY;
use tracker::ExternalId;
use tracker::RemoteTracker;
use tracker::TrackerError;

const WHOAMI: &str = "/api/v4/whoami";

fn whoami_body(scopes: &[&str]) -> Value {
    json!({"entity": {
        "member": {"id": "m-1", "mention_name": "ada", "name": "Ada"},
        "workspace": {"id": "w-1", "slug": "scratch-workspace",
                      "name": "Scratch Workspace"},
        "authorization": {"scopes": scopes},
        "default_workflow_state": {"id": 500_000_007}
    }})
}

fn without_an_unstarted_state() -> Value {
    let mut catalogue = catalogue_value();
    for state in catalogue["workflows"][0]["states"]
        .as_array_mut()
        .expect("states")
    {
        if state["type"] == "unstarted" {
            state["type"] = json!("started");
        }
    }
    catalogue
}

#[test]
fn a_failed_whoami_during_create_is_reported_and_nothing_is_sent() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(WHOAMI),
        json_route(401, &json!({"message": "Unauthorized"})),
    );
    let client = client_over(
        &server,
        catalogue_from(&without_an_unstarted_state()),
        Some(TEAM_KEY),
    );

    let error = client
        .create("A story", "Body", "story")
        .expect_err("whoami failed");

    let TrackerError::Retryable { detail } = &error else {
        panic!("a failed lookup before the send is retryable: {error:?}");
    };
    assert!(detail.contains("whoami"), "{detail}");
    assert!(!detail.contains("init discover"), "{detail}");
    assert_eq!(server.hits(&RequestKey::post(&path("/stories"))), 0);
}

#[test]
fn a_failed_scope_lookup_is_not_remembered() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::new("PATCH", &path("/stories/14")),
        json_route(403, &json!({"message": "Insufficient scope"})),
    );
    server.route(
        RequestKey::get(WHOAMI),
        Route::Sequence(vec![
            json_route(401, &json!({"message": "Unauthorized"})),
            json_route(200, &whoami_body(&["read"])),
        ]),
    );
    let client = client(&server);
    let story = ExternalId::new("sc-14".to_owned());

    client.update(&story, None, "Body").expect_err("refused");
    let second = client.update(&story, None, "Body").expect_err("refused");

    assert!(
        second.to_string().contains("the token has only the"),
        "{second}"
    );
}
