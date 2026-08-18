//! The catalogue-backed state resolver: case-insensitive, trimmed matching, and
//! the ambiguity a name shared by two catalogue states produces.

#![allow(clippy::expect_used)]

use linear_client::filter::StateResolver as _;
use linear_client::CatalogueStates;
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
