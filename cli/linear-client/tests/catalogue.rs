//! The catalogue-backed state resolver: case-insensitive, trimmed matching, and
//! the ambiguity a name shared by two catalogue states produces.

#![allow(clippy::expect_used)]

use linear_client::filter::StateResolver as _;
use linear_client::filter::TeamResolver as _;
use linear_client::CatalogueStates;
use linear_client::CatalogueTeam;
use serde_json::json;

fn catalogue() -> serde_json::Value {
    json!({
        "team": { "id": "team-1", "key": "ENG", "name": "Engineering" },
        "workflowStates": [
            { "id": "s1", "name": "Todo" },
            { "id": "s2", "name": "In Progress" },
            { "id": "s3", "name": "In Review" },
            { "id": "s4", "name": "In Review" }
        ]
    })
}

#[test]
fn a_unique_name_resolves_case_insensitively_and_trimmed() {
    let states = CatalogueStates::from_catalogue(&catalogue());
    assert_eq!(states.resolve("  in progress  "), Some("s2".to_owned()));
}

#[test]
fn an_unknown_name_resolves_to_nothing() {
    let states = CatalogueStates::from_catalogue(&catalogue());
    assert_eq!(states.resolve("Backlog"), None);
    assert!(states.resolve_all("Backlog").is_empty());
}

#[test]
fn a_shared_display_name_resolves_to_every_matching_id() {
    let states = CatalogueStates::from_catalogue(&catalogue());

    assert_eq!(
        states.resolve("In Review"),
        None,
        "an ambiguous name does not silently pick one"
    );
    assert_eq!(
        states.resolve_all("in review"),
        vec!["s3".to_owned(), "s4".to_owned()]
    );
}

#[test]
fn an_absent_catalogue_loads_an_empty_resolver() {
    let states = CatalogueStates::load(std::path::Path::new("/nonexistent"));
    assert!(states.resolve_all("anything").is_empty());
}

#[test]
fn catalogue_team_resolves_the_matching_key_to_its_uuid() {
    let team = CatalogueTeam::from_catalogue(&catalogue());
    assert_eq!(team.resolve("ENG"), Some("team-1".to_owned()));
    assert_eq!(team.resolve("  ENG  "), Some("team-1".to_owned()));
}

#[test]
fn catalogue_team_resolves_a_non_matching_key_to_nothing() {
    let team = CatalogueTeam::from_catalogue(&catalogue());
    assert_eq!(team.resolve("OTHER"), None);
}

#[test]
fn catalogue_team_from_an_absent_catalogue_resolves_nothing() {
    let team = CatalogueTeam::load(std::path::Path::new("/nonexistent"));
    assert_eq!(team.resolve("ENG"), None);
}

#[test]
fn catalogue_team_with_a_blank_key_resolves_nothing() {
    let team = CatalogueTeam::from_catalogue(&json!({
        "team": { "id": "team-1", "key": "" }
    }));
    assert_eq!(team.resolve(""), None, "a blank key must not resolve");
    assert_eq!(team.resolve("ENG"), None);
}

#[test]
fn a_grown_catalogue_resolves_every_team_in_the_teams_array() {
    let grown = json!({
        "team": { "id": "team-1", "key": "ENG", "name": "Engineering" },
        "teams": [
            { "id": "team-1", "key": "ENG", "name": "Engineering" },
            { "id": "team-2", "key": "OPS", "name": "Operations" }
        ],
        "workflowStates": []
    });
    let team = CatalogueTeam::from_catalogue(&grown);

    assert_eq!(team.resolve("ENG"), Some("team-1".to_owned()));
    assert_eq!(team.resolve("OPS"), Some("team-2".to_owned()));
}

#[test]
fn catalogued_lists_every_team_for_the_multi_team_keyed_read() {
    let grown = json!({
        "team": { "id": "team-1", "key": "ENG", "name": "Engineering" },
        "teams": [
            { "id": "team-1", "key": "ENG", "name": "Engineering" },
            { "id": "team-2", "key": "OPS", "name": "Operations" }
        ]
    });
    let mut listed = CatalogueTeam::from_catalogue(&grown).catalogued();
    listed.sort();

    assert_eq!(
        listed,
        vec![
            ("ENG".to_owned(), "team-1".to_owned()),
            ("OPS".to_owned(), "team-2".to_owned()),
        ]
    );
}

#[test]
fn a_pre_upgrade_catalogue_resolves_base_only_and_lists_the_base() {
    // No `teams` array: a newer binary reading a pre-upgrade file still
    // resolves and lists the base team.
    let pre_upgrade = json!({
        "team": { "id": "team-1", "key": "ENG", "name": "Engineering" }
    });
    let team = CatalogueTeam::from_catalogue(&pre_upgrade);

    assert_eq!(team.resolve("ENG"), Some("team-1".to_owned()));
    assert_eq!(
        team.catalogued(),
        vec![("ENG".to_owned(), "team-1".to_owned())]
    );
}
