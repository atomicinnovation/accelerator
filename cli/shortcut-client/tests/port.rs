//! The `RemoteTracker` operations against a mock v4 API serving the committed
//! live fixtures.

#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

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
use support::client::fixture;
use support::client::fixture_parts;
use support::client::json_route;
use support::client::path;
use support::client::OTHER_TEAM;
use support::client::TEAM;
use support::client::TEAM_KEY;
use support::client::TO_DO;
use tracker::Completeness;
use tracker::EntityScope;
use tracker::ExternalId;
use tracker::FieldResolution;
use tracker::Located;
use tracker::RemoteTimestamp;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker::TrackerError;
use tracker::ValidationOutcome;

fn id(text: &str) -> ExternalId {
    ExternalId::new(text.to_owned())
}

fn last_json(server: &MockHTTPServer, key: &RequestKey) -> Value {
    serde_json::from_slice(&server.last_body(key).expect("a request body"))
        .expect("a JSON body")
}

fn keyed(base: Option<&str>, additional: &[&str]) -> EntityScope {
    EntityScope::Keyed {
        base: base.map(str::to_owned),
        additional: additional.iter().map(|key| (*key).to_owned()).collect(),
    }
}

fn scope(entities: EntityScope, filters: &[(&str, &str)]) -> SearchScope {
    SearchScope {
        entities,
        filters: filters
            .iter()
            .map(|(field, value)| ((*field).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

#[test]
fn create_makes_a_feature_story_in_the_first_unstarted_state_by_position() {
    let server = MockHTTPServer::start();
    let stories = RequestKey::post(&path("/stories"));
    server.route(stories.clone(), fixture("story-create"));

    let created = client(&server)
        .create("Contract story", "Body\n", "story")
        .expect("creates");

    assert_eq!(created, id("sc-14"));
    let sent = last_json(&server, &stories);
    assert_eq!(sent["story_type"], json!("feature"));
    assert_eq!(sent["team_id"], json!(TEAM));
    assert_eq!(sent["workflow_state_id"], json!(TO_DO));
    assert_eq!(sent["description"], json!("Body\n"));
}

#[test]
fn create_with_kind_epic_makes_an_epic_with_no_team() {
    let server = MockHTTPServer::start();
    let epics = RequestKey::post(&path("/epics"));
    server.route(epics.clone(), fixture("epic-create"));

    let created = client(&server)
        .create("Contract epic", "Body\n", "epic")
        .expect("creates");

    assert_eq!(created, id("epic-17"));
    let sent = last_json(&server, &epics);
    assert!(sent.get("team_id").is_none(), "{sent}");
    assert_eq!(sent["name"], json!("Contract epic"));
}

#[test]
fn create_with_an_unknown_kind_sends_nothing_and_is_rejected() {
    let server = MockHTTPServer::start();

    let error = client(&server)
        .create("t", "b", "initiative")
        .expect_err("refused");

    assert!(matches!(error, TrackerError::Rejected { .. }), "{error:?}");
    assert_eq!(server.hits(&RequestKey::post(&path("/stories"))), 0);
}

#[test]
fn a_created_answer_without_a_usable_id_is_terminal() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::post(&path("/stories")),
        json_route(201, &json!({"entity": {"name": "x"}})),
    );

    let error = client(&server).create("t", "b", "").expect_err("unusable");

    assert!(matches!(error, TrackerError::Terminal { .. }), "{error:?}");
}

#[test]
fn a_story_create_answered_500_is_sent_once_and_terminal() {
    let server = MockHTTPServer::start();
    let stories = RequestKey::post(&path("/stories"));
    server.route(stories.clone(), Route::Status(500));

    let error = client(&server).create("t", "b", "").expect_err("fails");

    assert!(matches!(error, TrackerError::Terminal { .. }), "{error:?}");
    assert_eq!(server.hits(&stories), 1);
}

#[test]
fn an_epic_create_answered_500_is_sent_once() {
    let server = MockHTTPServer::start();
    let epics = RequestKey::post(&path("/epics"));
    server.route(epics.clone(), Route::Status(500));

    let _ = client(&server).create("t", "b", "epic");

    assert_eq!(server.hits(&epics), 1);
}

#[test]
fn a_create_answered_429_then_201_is_sent_twice() {
    let server = MockHTTPServer::start();
    let stories = RequestKey::post(&path("/stories"));
    server.route(
        stories.clone(),
        Route::Sequence(vec![Route::Status(429), fixture("story-create")]),
    );

    assert_eq!(
        client(&server).create("t", "b", "").expect("creates"),
        id("sc-14")
    );
    assert_eq!(server.hits(&stories), 2);
}

#[test]
fn update_sends_the_description_and_the_title_only_when_given() {
    let server = MockHTTPServer::start();
    let story = RequestKey::new("PATCH", &path("/stories/14"));
    server.route(story.clone(), fixture("story-transition"));
    let tracker = client(&server);

    tracker
        .update(&id("sc-14"), None, "New body\n")
        .expect("updates");
    assert_eq!(
        last_json(&server, &story),
        json!({"description": "New body\n"})
    );

    tracker
        .update(&id("sc-14"), Some("New title"), "B\n")
        .expect("updates");
    assert_eq!(
        last_json(&server, &story),
        json!({"name": "New title", "description": "B\n"})
    );

    tracker
        .retitle(&id("sc-14"), "Only title")
        .expect("retitles");
    assert_eq!(last_json(&server, &story), json!({"name": "Only title"}));
}

#[test]
fn an_epic_update_reaches_the_epic() {
    let server = MockHTTPServer::start();
    let epic = RequestKey::new("PATCH", &path("/epics/17"));
    server.route(epic.clone(), fixture("epic-show-full"));

    client(&server)
        .update(&id("epic-17"), None, "B\n")
        .expect("updates");

    assert_eq!(server.hits(&epic), 1);
}

#[test]
fn an_update_answered_500_then_429_is_terminal() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::new("PATCH", &path("/stories/14")),
        Route::Sequence(vec![Route::Status(500), Route::Status(429)]),
    );

    let error = client(&server)
        .update(&id("sc-14"), None, "B\n")
        .expect_err("fails");

    assert!(matches!(error, TrackerError::Terminal { .. }), "{error:?}");
}

#[test]
fn an_update_rate_limited_throughout_is_retryable() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::new("PATCH", &path("/stories/14")),
        Route::Status(429),
    );

    let error = client(&server)
        .update(&id("sc-14"), None, "B\n")
        .expect_err("fails");

    assert!(matches!(error, TrackerError::Retryable { .. }), "{error:?}");
}

#[test]
fn an_update_to_an_unparseable_key_is_rejected_before_sending() {
    let server = MockHTTPServer::start();

    let error = client(&server)
        .update(&id("ENG-1"), None, "B\n")
        .expect_err("refused");

    assert!(matches!(error, TrackerError::Rejected { .. }), "{error:?}");
}

#[test]
fn show_projects_the_name_and_description_under_the_requested_key() {
    let server = MockHTTPServer::start();
    let story = RequestKey::get(&path("/stories/14"));
    server.route(story.clone(), fixture("story-show-fields"));
    let (_, shown) = fixture_parts("story-show-fields");

    let issue = client(&server).show(&id("SC-14")).expect("shows");

    assert_eq!(issue.key, id("SC-14"));
    let description = shown["entity"]["description"].as_str().expect("text");
    assert_eq!(
        issue.body,
        format!("Contract story\n{}", description.trim_end_matches('\n'))
            + "\n"
    );
    assert_eq!(
        issue.updated,
        RemoteTimestamp::Reported("2026-11-14T08:21:34.384Z".to_owned())
    );
    assert!(server
        .last_query(&story)
        .expect("a query")
        .contains("fields=id%2Cname%2Cdescription%2Cupdated_at%2Capp_url"));
}

#[test]
fn locate_reports_a_404_as_not_found() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path("/stories/999")),
        fixture("error-404-story"),
    );

    assert_eq!(
        client(&server).locate(&id("sc-999")).expect("answers"),
        Located::NotFound
    );
}

#[test]
fn a_read_of_an_unparseable_key_is_retryable() {
    let server = MockHTTPServer::start();

    for error in [
        client(&server).show(&id("ENG-1")).expect_err("refused"),
        client(&server).locate(&id("ENG-1")).expect_err("refused"),
    ] {
        assert!(matches!(error, TrackerError::Retryable { .. }), "{error:?}");
    }
}

#[test]
fn fetch_all_reads_stories_by_id_including_archived_ones() {
    let server = MockHTTPServer::start();
    let stories = RequestKey::get(&path("/stories"));
    server.route(stories.clone(), fixture("stories-by-ids-include-archived"));

    let outcome = client(&server)
        .fetch_all(&[id("sc-14"), id("SC-16"), id("sc-999999999")])
        .expect("reads");

    let found: Vec<&str> =
        outcome.found.iter().map(|(key, _)| key.as_str()).collect();
    assert!(
        found.contains(&"sc-14") && found.contains(&"SC-16"),
        "{found:?}"
    );
    assert_eq!(outcome.absent, vec![id("sc-999999999")]);
    assert_eq!(outcome.completeness, Completeness::Complete);
    let query = server.last_query(&stories).expect("a query");
    assert!(query.contains("filter=include_archived"), "{query}");
    assert!(query.contains("limit=100"), "{query}");
}

#[test]
fn fetch_all_chunks_stories_by_one_hundred() {
    let server = MockHTTPServer::start();
    let stories = RequestKey::get(&path("/stories"));
    server.route(stories.clone(), json_route(200, &json!({"entities": []})));
    let tracker = client(&server);

    let hundred: Vec<ExternalId> =
        (1..=100).map(|n| id(&format!("sc-{n}"))).collect();
    tracker.fetch_all(&hundred).expect("reads");
    assert_eq!(server.hits(&stories), 1);

    let hundred_and_one: Vec<ExternalId> =
        (1..=101).map(|n| id(&format!("sc-{n}"))).collect();
    tracker.fetch_all(&hundred_and_one).expect("reads");
    assert_eq!(server.hits(&stories), 3);
    let queries = server.queries(&stories);
    assert!(
        queries
            .last()
            .and_then(Option::as_deref)
            .is_some_and(|query| query.contains("ids=101&")),
        "{queries:?}"
    );
}

#[test]
fn fetch_all_reads_each_epic_on_its_own() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path("/stories")),
        fixture("stories-by-ids-include-archived"),
    );
    server.route(
        RequestKey::get(&path("/epics/17")),
        fixture("epic-show-full"),
    );
    server.route(
        RequestKey::get(&path("/epics/18")),
        fixture("error-404-story"),
    );

    let outcome = client(&server)
        .fetch_all(&[id("sc-14"), id("epic-17"), id("epic-18")])
        .expect("reads");

    let found: Vec<&str> =
        outcome.found.iter().map(|(key, _)| key.as_str()).collect();
    assert!(
        found.contains(&"sc-14") && found.contains(&"epic-17"),
        "{found:?}"
    );
    assert_eq!(outcome.absent, vec![id("epic-18")]);
}

#[test]
fn a_failed_chunk_leaves_its_ids_indeterminate() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get(&path("/stories")), Route::Status(500));

    let outcome = client(&server).fetch_all(&[id("sc-14")]).expect("reads");

    assert_eq!(outcome.indeterminate, vec![id("sc-14")]);
    assert!(!outcome.completeness.is_complete());
}

#[test]
fn an_unrequested_id_in_the_answer_leaves_the_unanswered_ids_indeterminate() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path("/stories")),
        json_route(200, &json!({"entities": [{"id": 77, "updated_at": "x"}]})),
    );

    let outcome = client(&server).fetch_all(&[id("sc-14")]).expect("reads");

    assert_eq!(outcome.indeterminate, vec![id("sc-14")]);
    assert!(outcome.absent.is_empty());
}

#[test]
fn an_unparseable_key_is_indeterminate() {
    let server = MockHTTPServer::start();

    let outcome = client(&server).fetch_all(&[id("ENG-1")]).expect("reads");

    assert_eq!(outcome.indeterminate, vec![id("ENG-1")]);
}

#[test]
fn search_keeps_only_the_scoped_teams_stories() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path(&format!("/teams/{TEAM}"))),
        fixture("team-by-uuid"),
    );
    let (_, mut results) = fixture_parts("search-team");
    let mut stranger = results["entities"][0].clone();
    stranger["id"] = json!(99);
    stranger["team"]["id"] = json!(OTHER_TEAM);
    results["entities"]
        .as_array_mut()
        .expect("entities")
        .push(stranger);
    server.route(
        RequestKey::get(&path("/stories/search")),
        json_route(200, &results),
    );

    let discovery = client(&server)
        .search(&scope(keyed(Some(TEAM), &[]), &[]))
        .expect("searches");

    assert_eq!(
        discovery
            .found
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        vec!["sc-14"]
    );
    assert_eq!(discovery.completeness, Completeness::Complete);
    let query = server
        .last_query(&RequestKey::get(&path("/stories/search")))
        .expect("a query");
    assert!(query.contains("query=team%3A%22Team+1%22"), "{query}");
}

#[test]
fn search_unions_one_query_per_filter_value() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path(&format!("/teams/{TEAM}"))),
        fixture("team-by-uuid"),
    );
    server.route(
        RequestKey::get(&path("/stories/search")),
        fixture("search-team"),
    );

    let discovery = client(&server)
        .search(&scope(
            keyed(Some(TEAM), &[]),
            &[("label", "a"), ("label", "b")],
        ))
        .expect("searches");

    assert_eq!(server.hits(&RequestKey::get(&path("/stories/search"))), 2);
    assert_eq!(
        discovery.found.len(),
        1,
        "the same story twice is one result"
    );
}

#[test]
fn a_team_that_no_longer_exists_is_unconfigured() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path(&format!("/teams/{TEAM}"))),
        fixture("team-unknown-404"),
    );

    let error = client(&server)
        .search(&scope(keyed(Some(TEAM), &[]), &[]))
        .expect_err("refused");

    assert!(
        matches!(&error, TrackerError::Unconfigured { detail } if detail.contains("init discover")),
        "{error:?}"
    );
}

#[test]
fn a_failed_team_read_is_retryable() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path(&format!("/teams/{TEAM}"))),
        Route::Status(500),
    );

    let error = client(&server)
        .search(&scope(keyed(Some(TEAM), &[]), &[]))
        .expect_err("fails");

    assert!(matches!(error, TrackerError::Retryable { .. }), "{error:?}");
}

#[test]
fn a_configured_team_whose_name_cannot_be_quoted_is_unconfigured() {
    let server = MockHTTPServer::start();
    let (_, mut team) = fixture_parts("team-by-uuid");
    team["entity"]["name"] = json!("Ops \"Night\"");
    server.route(
        RequestKey::get(&path(&format!("/teams/{TEAM}"))),
        json_route(200, &team),
    );

    let error = client(&server)
        .search(&scope(keyed(Some(TEAM), &[]), &[]))
        .expect_err("refused");

    assert!(
        matches!(error, TrackerError::Unconfigured { .. }),
        "{error:?}"
    );
}

#[test]
fn an_enumerated_team_whose_name_cannot_be_quoted_is_skipped() {
    let server = MockHTTPServer::start();
    let (_, mut team) = fixture_parts("team-by-uuid");
    team["entity"]["name"] = json!("Ops \"Night\"");
    server.route(
        RequestKey::get(&path(&format!("/teams/{OTHER_TEAM}"))),
        json_route(200, &team),
    );
    server.route(
        RequestKey::get(&path(&format!("/teams/{TEAM}"))),
        fixture("team-by-uuid"),
    );
    server.route(
        RequestKey::get(&path("/stories/search")),
        fixture("search-team"),
    );

    let discovery = client(&server)
        .search(&scope(keyed(None, &[OTHER_TEAM, TEAM]), &[]))
        .expect("searches the other teams");

    assert_eq!(discovery.found.len(), 1);
    assert!(!discovery.completeness.is_complete());
}

#[test]
fn enumeration_keys_teams_by_team_key_with_the_uuid_as_identifier() {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path("/teams")),
        json_route(
            200,
            &json!({"entities": [
                {"id": TEAM, "name": "Team 1", "mention_name": TEAM_KEY},
                {"id": "u-ops-1", "name": "Ops", "mention_name": null},
                {"id": "u-ops-2", "name": "Ops", "mention_name": null},
                {"id": "u-web", "name": "Web", "mention_name": null}
            ]}),
        ),
    );

    let visible = client(&server)
        .enumerate_visible_entities()
        .expect("enumerates");

    let keys: Vec<(&str, &str)> = visible
        .iter()
        .map(|entity| (entity.key.as_str(), entity.identifier.as_str()))
        .collect();
    assert_eq!(
        keys,
        vec![
            (TEAM_KEY, TEAM),
            ("u-ops-1", "u-ops-1"),
            ("u-ops-2", "u-ops-2"),
            ("Web", "u-web")
        ]
    );
}

#[test]
fn a_failed_enumeration_is_retryable() {
    let server = MockHTTPServer::start();
    server.route(RequestKey::get(&path("/teams")), Route::Status(500));

    assert!(matches!(
        client(&server).enumerate_visible_entities(),
        Err(TrackerError::Retryable { .. })
    ));
}

#[test]
fn a_base_only_scope_resolves_its_team_key_to_the_uuid() {
    let server = MockHTTPServer::start();

    let resolved = client(&server)
        .resolve_scope(&scope(keyed(Some(TEAM_KEY), &[]), &[]))
        .expect("resolves");

    assert_eq!(resolved.entities, keyed(Some(TEAM), &[]));
}

#[test]
fn a_broadened_scope_keeps_its_configured_keys() {
    let server = MockHTTPServer::start();
    let requested = scope(keyed(Some(TEAM_KEY), &["web", "uncatalogued"]), &[]);

    let resolved = client(&server).resolve_scope(&requested).expect("resolves");

    assert_eq!(resolved, requested);
}

#[test]
fn a_colliding_additional_team_key_is_refused_with_the_recovery() {
    let server = MockHTTPServer::start();
    let mut value = catalogue_value();
    value["teams"]
        .as_array_mut()
        .expect("teams")
        .extend([
            json!({"id": "00000000-0000-4000-8000-0000000000a1", "name": "Ops", "mention_name": null}),
            json!({"id": "00000000-0000-4000-8000-0000000000a2", "name": "Ops", "mention_name": null}),
        ]);
    let tracker = client_over(&server, catalogue_from(&value), Some(TEAM_KEY));

    for requested in [
        scope(keyed(Some(TEAM_KEY), &["Ops"]), &[]),
        scope(keyed(Some("Ops"), &[]), &[]),
    ] {
        let error = tracker.resolve_scope(&requested).expect_err("refused");
        assert!(error.detail.contains("init discover"), "{error}");
        assert!(error.detail.contains("mention name"), "{error}");
    }
}

#[test]
fn an_absent_base_team_key_is_refused_naming_list_teams() {
    let server = MockHTTPServer::start();

    let error = client(&server)
        .resolve_scope(&scope(keyed(None, &[]), &[]))
        .expect_err("refused");

    assert!(error.detail.contains("init list-teams"), "{error}");
}

#[test]
fn a_state_filter_is_checked_against_the_workflows_in_scope() {
    let server = MockHTTPServer::start();
    let tracker = client(&server);

    assert!(tracker
        .resolve_scope(&scope(
            keyed(Some(TEAM_KEY), &[]),
            &[("state", "To Do")]
        ))
        .is_ok());
    assert!(tracker
        .resolve_scope(&scope(
            keyed(Some(TEAM_KEY), &["web"]),
            &[("state", "Brief")]
        ))
        .is_ok());
    let unknown = tracker
        .resolve_scope(&scope(
            keyed(Some(TEAM_KEY), &[]),
            &[("state", "Brief")],
        ))
        .expect_err("Brief is another team's state");
    assert!(unknown.detail.contains("init discover"), "{unknown}");
    assert!(tracker
        .resolve_scope(&scope(
            EntityScope::WholeWorkspace,
            &[("state", "Brief")]
        ))
        .is_ok());
}

#[test]
fn a_state_filter_over_an_uncatalogued_team_is_refused() {
    let server = MockHTTPServer::start();

    let error = client(&server)
        .resolve_scope(&scope(
            keyed(Some(TEAM_KEY), &["uncatalogued"]),
            &[("state", "To Do")],
        ))
        .expect_err("refused");

    assert!(error.detail.contains("init discover"), "{error}");
}

#[test]
fn an_epic_type_filter_is_refused() {
    let server = MockHTTPServer::start();

    assert!(client(&server)
        .resolve_scope(&scope(keyed(Some(TEAM_KEY), &[]), &[("type", "epic")]))
        .is_err());
}

#[test]
fn preview_resolves_the_type_and_the_team() {
    let server = MockHTTPServer::start();
    let tracker = client(&server);

    let story = tracker.preview_create("bug").expect("previews");
    assert_eq!(
        story.issue_type,
        FieldResolution::Resolved("bug".to_owned())
    );
    assert_eq!(
        story.project,
        FieldResolution::Resolved("Team 1".to_owned())
    );

    let default = tracker.preview_create("").expect("previews");
    assert_eq!(default.issue_type, FieldResolution::Unset);

    let unknown = tracker.preview_create("initiative").expect("previews");
    assert_eq!(
        unknown.issue_type,
        FieldResolution::Unresolvable("initiative".to_owned())
    );

    let epic = tracker.preview_create("epic").expect("previews");
    assert_eq!(
        epic.issue_type,
        FieldResolution::Resolved("epic".to_owned())
    );
    assert_eq!(epic.project, FieldResolution::Unset);

    let misconfigured = client_over(
        &server,
        catalogue_from(&catalogue_value()),
        Some("nobody"),
    );
    assert_eq!(
        misconfigured
            .preview_create("story")
            .expect("previews")
            .project,
        FieldResolution::Unresolvable("nobody".to_owned())
    );
}

#[test]
fn validate_update_refuses_only_an_empty_title() {
    let server = MockHTTPServer::start();
    let tracker = client(&server);

    assert_eq!(
        tracker.validate_update(&id("PREVIEW-1"), Some("t"), Some("b")),
        ValidationOutcome::Valid
    );
    assert!(matches!(
        tracker.validate_update(&id("sc-1"), Some(" "), None),
        ValidationOutcome::Rejected { .. }
    ));
}
