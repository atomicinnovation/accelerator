//! The port's contract properties against a mock, in the default profile, for
//! a story subject and an epic subject.
//!
//! Each property runs against a fresh server scripted from the committed live
//! fixtures. Shortcut ids never move, so the moved-issue property does not
//! apply; the inventory test fails if any other exported property is left out.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::path::Path;

use http_test_support::MockHTTPServer;
use http_test_support::RequestKey;
use http_test_support::Route;
use serde_json::json;
use shortcut_client::ShortcutClient;
use shortcut_client::Url;
use support::client::catalogue_from;
use support::client::catalogue_value;
use support::client::client;
use support::client::fixture;
use support::client::json_route;
use support::client::path;
use support::client::transport_with;
use support::client::TEAM;
use support::client::TEAM_KEY;
use tracker::Ceiling;
use tracker::CreatePreview;
use tracker::EntityScope;
use tracker::ExternalId;
use tracker::FieldResolution;
use tracker::RemoteTracker;
use tracker::SearchScope;
use tracker_support::TransportConfig;
use tracker_test_support::contract::a_failing_read_is_retryable_property;
use tracker_test_support::contract::create_then_show_round_trips_property;
use tracker_test_support::contract::fetch_all_partitions_totally_property;
use tracker_test_support::contract::locate_finds_a_created_issue_property;
use tracker_test_support::contract::preview_create_makes_no_mutation_property;
use tracker_test_support::contract::preview_create_resolves_fields_property;
use tracker_test_support::contract::retitle_keeps_the_description_property;
use tracker_test_support::contract::search_reports_truncation_property;
use tracker_test_support::contract::unaccounted_id_is_indeterminate_not_absent_property;
use tracker_test_support::contract::update_replaces_whole_content_property;
use tracker_test_support::contract::update_without_a_title_keeps_the_title_property;
use tracker_test_support::contract::validate_update_reports_outcome_property;
use tracker_test_support::contract::ContractSubject;

const RUN_HERE: [&str; 12] = [
    "create_then_show_round_trips_property",
    "locate_finds_a_created_issue_property",
    "update_replaces_whole_content_property",
    "retitle_keeps_the_description_property",
    "update_without_a_title_keeps_the_title_property",
    "fetch_all_partitions_totally_property",
    "unaccounted_id_is_indeterminate_not_absent_property",
    "a_failing_read_is_retryable_property",
    "search_reports_truncation_property",
    "preview_create_makes_no_mutation_property",
    "preview_create_resolves_fields_property",
    "validate_update_reports_outcome_property",
];
const NOT_APPLICABLE: [&str; 1] =
    ["a_moved_issue_is_found_only_under_its_current_key_property"];

#[derive(Clone, Copy)]
enum Entity {
    Story,
    Epic,
}

impl Entity {
    const fn collection(self) -> &'static str {
        match self {
            Self::Story => "stories",
            Self::Epic => "epics",
        }
    }

    const fn created_fixture(self) -> &'static str {
        match self {
            Self::Story => "story-create",
            Self::Epic => "epic-create",
        }
    }

    const fn number(self) -> u64 {
        match self {
            Self::Story => 14,
            Self::Epic => 17,
        }
    }

    const fn prefix(self) -> &'static str {
        match self {
            Self::Story => "sc",
            Self::Epic => "epic",
        }
    }

    const fn kind(self) -> &'static str {
        match self {
            Self::Story => "",
            Self::Epic => "epic",
        }
    }

    fn entity_path(self) -> String {
        path(&format!("/{}/{}", self.collection(), self.number()))
    }
}

struct Subject {
    entity: Entity,
    client: ShortcutClient,
}

impl ContractSubject for Subject {
    fn tracker(&self) -> &dyn RemoteTracker {
        &self.client
    }

    fn unaccountable_id(&self) -> ExternalId {
        ExternalId::new(format!(
            "{}-{}",
            self.entity.prefix(),
            self.entity.number()
        ))
    }

    fn unreadable_id(&self) -> ExternalId {
        ExternalId::new(format!(
            "{}-{}",
            self.entity.prefix(),
            self.entity.number()
        ))
    }

    fn truncating_scope(&self) -> SearchScope {
        SearchScope {
            entities: EntityScope::Keyed {
                base: Some(TEAM.to_owned()),
                additional: Vec::new(),
            },
            filters: Vec::new(),
        }
    }

    fn preview_kind(&self) -> String {
        self.entity.kind().to_owned()
    }
}

fn shown(entity: Entity, title: &str, description: &str) -> Route {
    json_route(
        200,
        &json!({"entity": {
            "id": entity.number(),
            "name": title,
            "description": description,
            "updated_at": "2026-11-14T08:21:34.384Z",
        }}),
    )
}

fn subject_over(entity: Entity, server: &MockHTTPServer) -> Subject {
    Subject {
        entity,
        client: client(server),
    }
}

fn creating(entity: Entity, shows: Vec<Route>) -> (MockHTTPServer, Subject) {
    let server = MockHTTPServer::start();
    server.route(
        RequestKey::post(&path(&format!("/{}", entity.collection()))),
        fixture(entity.created_fixture()),
    );
    server.route(
        RequestKey::get(&entity.entity_path()),
        Route::Sequence(shows),
    );
    server.route(
        RequestKey::new("PATCH", &entity.entity_path()),
        json_route(200, &json!({"entity": {"id": entity.number()}})),
    );
    let subject = subject_over(entity, &server);
    (server, subject)
}

fn run_every_property(entity: Entity) -> usize {
    run_write_properties(entity)
        + run_read_properties(entity)
        + run_preview_properties(entity)
}

fn run_write_properties(entity: Entity) -> usize {
    let mut executed = 0;

    let (_server, subject) = creating(
        entity,
        vec![shown(entity, "Contract title", "Contract body\n")],
    );
    create_then_show_round_trips_property(&subject);
    executed += 1;

    let (_server, subject) = creating(
        entity,
        vec![shown(entity, "Contract title", "Contract body\n")],
    );
    locate_finds_a_created_issue_property(&subject);
    executed += 1;

    let (_server, subject) = creating(
        entity,
        vec![
            shown(entity, "Original title", "Original body\n"),
            shown(entity, "Updated title", "Updated body\n"),
        ],
    );
    update_replaces_whole_content_property(&subject);
    executed += 1;

    let (_server, subject) = creating(
        entity,
        vec![
            shown(entity, "Original title", "Original body\n"),
            shown(entity, "Retitled", "Original body\n"),
        ],
    );
    retitle_keeps_the_description_property(&subject);
    executed += 1;

    let (_server, subject) = creating(
        entity,
        vec![
            shown(entity, "Original title", "Original body\n"),
            shown(entity, "Original title", "Updated body\n"),
        ],
    );
    update_without_a_title_keeps_the_title_property(&subject);
    executed += 1;

    executed
}

fn run_read_properties(entity: Entity) -> usize {
    let mut executed = 0;

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
    let ids = [
        ExternalId::new("sc-14".to_owned()),
        ExternalId::new("sc-99".to_owned()),
        ExternalId::new("epic-17".to_owned()),
        ExternalId::new("epic-18".to_owned()),
        ExternalId::new("ENG-1".to_owned()),
    ];
    fetch_all_partitions_totally_property(&subject_over(entity, &server), &ids);
    executed += 1;

    let server = MockHTTPServer::start();
    server.route(RequestKey::get(&path("/stories")), Route::Status(500));
    server.route(RequestKey::get(&path("/epics/17")), Route::Status(500));
    unaccounted_id_is_indeterminate_not_absent_property(&subject_over(
        entity, &server,
    ));
    executed += 1;

    let server = MockHTTPServer::start();
    server.route(RequestKey::get(&entity.entity_path()), Route::Status(500));
    a_failing_read_is_retryable_property(&subject_over(entity, &server));
    executed += 1;

    let server = MockHTTPServer::start();
    server.route(
        RequestKey::get(&path(&format!("/teams/{TEAM}"))),
        fixture("team-by-uuid"),
    );
    server.route(
        RequestKey::get(&path("/stories/search")),
        json_route(
            200,
            &json!({"entities": [], "next_page_url":
                format!("{}{}?cursor=2", server.base_url(), path("/stories/search"))}),
        ),
    );
    let capped = Subject {
        entity,
        client: ShortcutClient::new(
            transport_with(TransportConfig {
                discovery_max_pages: Ceiling::Bounded(1),
                ..TransportConfig::default()
            }),
            Url::parse(&server.base_url()).expect("parses"),
            catalogue_from(&catalogue_value()),
            Some(TEAM_KEY.to_owned()),
        ),
    };
    search_reports_truncation_property(&capped);
    executed += 1;

    executed
}

fn run_preview_properties(entity: Entity) -> usize {
    let mut executed = 0;

    let server = MockHTTPServer::start();
    let previewing = subject_over(entity, &server);
    preview_create_makes_no_mutation_property(&previewing);
    executed += 1;
    let expected = match entity {
        Entity::Story => CreatePreview {
            project: FieldResolution::Resolved("Team 1".to_owned()),
            issue_type: FieldResolution::Unset,
        },
        Entity::Epic => CreatePreview {
            project: FieldResolution::Unset,
            issue_type: FieldResolution::Resolved("epic".to_owned()),
        },
    };
    preview_create_resolves_fields_property(&previewing, &expected);
    executed += 1;
    assert_eq!(
        server.hits(&RequestKey::post(&path(&format!(
            "/{}",
            entity.collection()
        )))),
        0,
        "a preview must make no create"
    );

    validate_update_reports_outcome_property(&previewing);
    executed += 1;

    executed
}

#[test]
fn every_applicable_property_holds_for_a_story_subject() {
    assert_eq!(run_every_property(Entity::Story), RUN_HERE.len());
}

#[test]
fn every_applicable_property_holds_for_an_epic_subject() {
    assert_eq!(run_every_property(Entity::Epic), RUN_HERE.len());
}

#[test]
fn every_exported_property_is_run_here_or_named_inapplicable() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tracker-test-support/src/contract.rs"),
    )
    .expect("the contract harness source");
    let exported: Vec<&str> = source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub fn "))
        .filter_map(|rest| rest.split('(').next())
        .filter(|name| name.ends_with("_property"))
        .collect();

    let missing: Vec<&&str> = exported
        .iter()
        .filter(|name| {
            !RUN_HERE.contains(name) && !NOT_APPLICABLE.contains(name)
        })
        .collect();
    assert!(
        missing.is_empty(),
        "properties not run offline: {missing:?}"
    );
    assert_eq!(exported.len(), RUN_HERE.len() + NOT_APPLICABLE.len());
}
