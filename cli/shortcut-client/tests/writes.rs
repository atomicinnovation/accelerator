//! The write operations: how often each request is sent, and the name
//! lookups each makes before it sends anything.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::time::Duration;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use serde_json::json;
use shortcut_client::story_state::resolve_story_state;
use shortcut_client::story_state::StateRefusal;
use shortcut_client::writes::OwnerRefusal;
use shortcut_client::writes::StoryUpdate;
use shortcut_client::writes::WriteError;
use shortcut_client::EpicId;
use shortcut_client::ShortcutClient;
use shortcut_client::ShortcutKey;
use shortcut_client::StoryId;
use shortcut_client::Url;
use support::client::catalogue_from;
use support::client::catalogue_value;
use support::client::client;
use support::client::json_route;
use support::client::path;
use support::client::transport_with;
use support::client::TEAM_KEY;
use support::client::TO_DO;
use support::client::WORKFLOW;
use tracker_support::TransportConfig;

const STORY: ShortcutKey = ShortcutKey::Story(StoryId::new(14));

fn story_path() -> String {
    path("/stories/14")
}

fn sent(server: &MockHTTPServer, key: &RequestKey) -> serde_json::Value {
    let body = server.last_body(key).expect("a request was sent");
    serde_json::from_slice(&body).expect("the body is JSON")
}

fn ok() -> Route {
    json_route(200, &json!({"entity": {"id": 14}}))
}

fn story_in(workflow: i64) -> Route {
    json_route(
        200,
        &json!({"entity": {"id": 14, "workflow": {"id": workflow}}}),
    )
}

fn timing_out(server: &MockHTTPServer) -> ShortcutClient {
    ShortcutClient::new(
        transport_with(TransportConfig {
            timeout: Duration::from_millis(200),
            ..TransportConfig::default()
        }),
        Url::parse(&server.base_url()).expect("parses"),
        catalogue_from(&catalogue_value()),
        Some(TEAM_KEY.to_owned()),
    )
}

#[test]
fn a_comment_answered_500_is_sent_once() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::post(&path("/stories/14/comments")),
        Route::Status(500),
    );

    let result = client(&server).add_comment(STORY, "Hello");

    assert!(result.is_err());
    assert_eq!(
        server.hits(&RequestKey::post(&path("/stories/14/comments"))),
        1
    );
}

#[test]
fn a_comment_that_times_out_is_sent_once() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::post(&path("/stories/14/comments")),
        Route::Stall(Duration::from_secs(5)),
    );

    let result = timing_out(&server).add_comment(STORY, "Hello");

    assert!(result.is_err());
    assert_eq!(
        server.hits(&RequestKey::post(&path("/stories/14/comments"))),
        1
    );
}

#[test]
fn a_comment_on_an_epic_goes_to_the_epic() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::post(&path("/epics/17/comments")),
        json_route(201, &json!({"entity": {"id": 18}})),
    );

    client(&server)
        .add_comment(ShortcutKey::Epic(EpicId::new(17)), "Hello")
        .expect("the comment is added");

    assert_eq!(
        sent(&server, &RequestKey::post(&path("/epics/17/comments"))),
        json!({"text": "Hello"})
    );
}

#[test]
fn a_link_answered_500_is_sent_once() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::new("PATCH", &story_path()), Route::Status(500));

    let result =
        client(&server).attach_link(StoryId::new(14), "https://example.com");

    assert!(result.is_err());
    assert_eq!(server.hits(&RequestKey::new("PATCH", &story_path())), 1);
}

#[test]
fn a_throttled_link_is_sent_again() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::new("PATCH", &story_path()),
        Route::Sequence(vec![Route::Status(429), ok()]),
    );

    client(&server)
        .attach_link(StoryId::new(14), "https://example.com")
        .expect("the link is added");

    assert_eq!(server.hits(&RequestKey::new("PATCH", &story_path())), 2);
    assert_eq!(
        sent(&server, &RequestKey::new("PATCH", &story_path())),
        json!({"external_links_add": ["https://example.com"]})
    );
}

#[test]
fn a_link_that_is_not_https_is_refused_unsent() {
    let server = MockHTTPServer::start();

    let result =
        client(&server).attach_link(StoryId::new(14), "http://example.com");

    assert!(matches!(result, Err(WriteError::NotHttps(_))));
    assert_eq!(server.hits(&RequestKey::new("PATCH", &story_path())), 0);
}

#[test]
fn an_update_answered_500_then_200_is_sent_twice() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::new("PATCH", &story_path()),
        Route::Sequence(vec![Route::Status(500), ok()]),
    );

    client(&server)
        .update_entity(
            STORY,
            &StoryUpdate {
                title: Some("New".to_owned()),
                ..StoryUpdate::default()
            },
        )
        .expect("the update lands");

    assert_eq!(server.hits(&RequestKey::new("PATCH", &story_path())), 2);
}

#[test]
fn an_update_sends_only_the_fields_given() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::new("PATCH", &story_path()), ok());

    client(&server)
        .update_entity(
            STORY,
            &StoryUpdate {
                title: Some("New".to_owned()),
                estimate: Some(3),
                ..StoryUpdate::default()
            },
        )
        .expect("the update lands");

    assert_eq!(
        sent(&server, &RequestKey::new("PATCH", &story_path())),
        json!({"estimate": 3, "name": "New"})
    );
}

#[test]
fn a_transition_answered_500_then_200_is_sent_twice() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get(&story_path()), story_in(WORKFLOW));
    server.route(
        RequestKey::new("PATCH", &story_path()),
        Route::Sequence(vec![Route::Status(500), ok()]),
    );

    client(&server)
        .transition(StoryId::new(14), "to do")
        .expect("the transition lands");

    assert_eq!(server.hits(&RequestKey::new("PATCH", &story_path())), 2);
    assert_eq!(
        sent(&server, &RequestKey::new("PATCH", &story_path())),
        json!({"workflow_state_id": TO_DO})
    );
}

#[test]
fn an_update_with_a_state_reads_the_story_first() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get(&story_path()), story_in(WORKFLOW));
    server.route(RequestKey::new("PATCH", &story_path()), ok());

    client(&server)
        .update_entity(
            STORY,
            &StoryUpdate {
                state: Some("To Do".to_owned()),
                ..StoryUpdate::default()
            },
        )
        .expect("the update lands");

    assert_eq!(server.hits(&RequestKey::get(&story_path())), 1);
    assert_eq!(
        sent(&server, &RequestKey::new("PATCH", &story_path())),
        json!({"workflow_state_id": TO_DO})
    );
}

#[test]
fn a_state_from_another_workflow_is_refused_listing_the_valid_ones() {
    let catalogue = catalogue_from(&catalogue_value());

    let refusal = resolve_story_state(&catalogue, WORKFLOW, "Brief")
        .expect_err("Brief belongs to the Design workflow");

    let StateRefusal::Unknown { valid, .. } = &refusal else {
        panic!("expected Unknown, got {refusal:?}");
    };
    assert!(valid.contains(&"To Do".to_owned()), "{valid:?}");
    assert!(refusal.to_string().contains("init discover"), "{refusal}");
}

#[test]
fn a_state_named_twice_in_one_workflow_is_ambiguous() {
    let mut value = catalogue_value();
    value["workflows"][0]["states"]
        .as_array_mut()
        .expect("states")
        .push(
            json!({"id": 9, "name": "to do", "type": "started", "position": 3}),
        );
    let catalogue = catalogue_from(&value);

    let refusal = resolve_story_state(&catalogue, WORKFLOW, "To Do")
        .expect_err("two states share the name");

    assert!(
        matches!(refusal, StateRefusal::Ambiguous { .. }),
        "{refusal:?}"
    );
}

#[test]
fn a_workflow_missing_from_the_catalogue_names_it_and_the_refresh() {
    let catalogue = catalogue_from(&catalogue_value());

    let refusal = resolve_story_state(&catalogue, 777, "To Do")
        .expect_err("the workflow is not catalogued");

    let message = refusal.to_string();
    assert!(message.contains("777"), "{message}");
    assert!(message.contains("init discover"), "{message}");
}

#[test]
fn an_unknown_owner_sends_no_update_and_lists_the_members() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path("/members")),
        json_route(
            200,
            &json!({"entities": [
                {"id": "m-1", "mention_name": "ada", "name": "Ada",
                 "disabled": false}
            ]}),
        ),
    );
    server.route(RequestKey::new("PATCH", &story_path()), ok());

    let result = client(&server).update_entity(
        STORY,
        &StoryUpdate {
            owner: Some("grace".to_owned()),
            ..StoryUpdate::default()
        },
    );

    let Err(WriteError::Owner(OwnerRefusal::Unknown { known, .. })) = result
    else {
        panic!("an unknown owner is refused");
    };
    assert_eq!(known, ["ada"]);
    assert_eq!(server.hits(&RequestKey::new("PATCH", &story_path())), 0);
}

#[test]
fn a_known_owner_is_sent_as_its_uuid() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path("/members")),
        json_route(
            200,
            &json!({"entities": [
                {"id": "m-1", "mention_name": "ada", "name": "Ada",
                 "disabled": false}
            ]}),
        ),
    );
    server.route(RequestKey::new("PATCH", &story_path()), ok());

    client(&server)
        .update_entity(
            STORY,
            &StoryUpdate {
                owner: Some("ada".to_owned()),
                ..StoryUpdate::default()
            },
        )
        .expect("the update lands");

    assert_eq!(
        sent(&server, &RequestKey::new("PATCH", &story_path())),
        json!({"owner_ids": ["m-1"]})
    );
}

#[test]
fn a_state_or_estimate_on_an_epic_is_refused_unsent() {
    let server = MockHTTPServer::start();
    let epic = ShortcutKey::Epic(EpicId::new(17));

    for update in [
        StoryUpdate {
            state: Some("To Do".to_owned()),
            ..StoryUpdate::default()
        },
        StoryUpdate {
            estimate: Some(1),
            ..StoryUpdate::default()
        },
    ] {
        let result = client(&server).update_entity(epic, &update);
        assert!(matches!(result, Err(WriteError::NotAStory(_))));
    }
    assert_eq!(
        server.hits(&RequestKey::new("PATCH", &path("/epics/17"))),
        0
    );
}
