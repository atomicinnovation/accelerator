#![allow(clippy::expect_used, clippy::panic)]

use serde_json::json;
use serde_json::Value;
use shortcut_client::catalogue::Catalogue;
use shortcut_client::catalogue::CatalogueError;
use shortcut_client::catalogue::StateType;
use shortcut_client::catalogue::TeamLookupError;
use shortcut_client::catalogue::SCHEMA_VERSION;

const MOBILE: &str = "8f1c2d3e-0000-4000-8000-000000000001";
const OPS_ONE: &str = "8f1c2d3e-0000-4000-8000-000000000002";
const OPS_TWO: &str = "8f1c2d3e-0000-4000-8000-000000000003";

fn document() -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "workspace": {"slug": "acme", "name": "Acme"},
        "teams": [
            {"id": MOBILE, "name": "Mobile Team", "mention_name": "mobile",
             "default_workflow_id": 500, "workflow_ids": [500, 600]},
            {"id": OPS_ONE, "name": "Ops", "mention_name": null,
             "default_workflow_id": 500, "workflow_ids": [500]},
            {"id": OPS_TWO, "name": "Ops", "mention_name": null,
             "default_workflow_id": 500, "workflow_ids": [500]}
        ],
        "workflows": [
            {"id": 500, "name": "Engineering", "states": [
                {"id": 501, "name": "Backlog", "type": "backlog", "position": 1},
                {"id": 502, "name": "Ready", "type": "unstarted", "position": 2},
                {"id": 503, "name": "Done", "type": "done", "position": 3}
            ]},
            {"id": 600, "name": "Design", "states": [
                {"id": 601, "name": "Brief", "type": "unstarted", "position": 1}
            ]}
        ]
    })
}

fn load(value: &Value) -> Result<Catalogue, CatalogueError> {
    Catalogue::parse(&value.to_string())
}

#[test]
fn a_well_formed_catalogue_loads() {
    let catalogue = load(&document()).expect("loads");

    assert_eq!(catalogue.slug().as_str(), "acme");
    assert_eq!(catalogue.teams().len(), 3);
    let workflow = catalogue.workflow(500).expect("catalogued");
    assert_eq!(workflow.states[1].kind, StateType::Unstarted);
}

#[test]
fn a_team_key_resolves_by_mention_name() {
    let catalogue = load(&document()).expect("loads");

    let team = catalogue.team_for_key("mobile").expect("resolves");

    assert_eq!(team.id, MOBILE);
    assert_eq!(team.workflow_ids, vec![500, 600]);
}

#[test]
fn a_null_mention_team_resolves_by_display_name() {
    let mut value = document();
    value["teams"][2]["name"] = json!("Platform");
    let catalogue = load(&value).expect("loads");

    assert_eq!(
        catalogue.team_for_key("Platform").expect("resolves").id,
        OPS_TWO
    );
}

#[test]
fn a_colliding_key_is_refused_with_the_recovery_steps() {
    let catalogue = load(&document()).expect("loads");

    let error = catalogue.team_for_key("Ops").expect_err("collides");

    assert!(matches!(error, TeamLookupError::Colliding { .. }));
    let message = error.to_string();
    for step in [
        "mention name",
        "accelerator shortcut init discover",
        "shortcut.team_key",
        "commit",
    ] {
        assert!(message.contains(step), "{step}: {message}");
    }
}

#[test]
fn an_unknown_key_lists_the_known_keys() {
    let catalogue = load(&document()).expect("loads");

    let error = catalogue.team_for_key("Mobile Team").expect_err("unknown");

    assert!(matches!(error, TeamLookupError::Unknown { .. }));
    assert!(error.to_string().contains("mobile"), "{error}");
}

#[test]
fn a_team_whose_name_holds_a_quote_still_loads() {
    let mut value = document();
    value["teams"][0]["name"] = json!("Mobile \"Night\" \\ Team");
    let catalogue = load(&value).expect("loads");

    assert!(catalogue.team_for_key("mobile").is_ok());
}

#[test]
fn a_value_that_could_redirect_a_request_is_refused_naming_its_field() {
    for (pointer, bad, field) in [
        ("/workspace/slug", json!("ac/me"), "workspace.slug"),
        ("/teams/0/id", json!("not-a-uuid"), "teams[0].id"),
        ("/workflows/0/id", json!(0), "workflows[0].id"),
        (
            "/workflows/0/states/1/id",
            json!(-4),
            "workflows[0].states[1].id",
        ),
    ] {
        let mut value = document();
        *value.pointer_mut(pointer).expect("present") = bad;

        let error = load(&value).expect_err("refused");

        assert!(
            matches!(&error, CatalogueError::Invalid { field: f, .. } if f == field),
            "{pointer}: {error:?}"
        );
        assert!(error.to_string().contains("init discover"), "{error}");
    }
}

#[test]
fn an_unknown_field_and_an_unseen_state_type_still_load() {
    let mut value = document();
    value["future_field"] = json!({"x": 1});
    value["teams"][0]["colour"] = json!("blue");
    value["workflows"][0]["states"][0]["type"] = json!("triage");

    let catalogue = load(&value).expect("loads");

    assert_eq!(
        catalogue.workflow(500).expect("catalogued").states[0].kind,
        StateType::Other("triage".to_owned())
    );
}

#[test]
fn a_rewrite_keeps_fields_this_client_does_not_know() {
    let mut previous = document();
    previous["future_field"] = json!("top");
    previous["teams"][0]["colour"] = json!("blue");
    previous["workflows"][0]["owner"] = json!("ada");
    let previous = load(&previous).expect("loads");
    let fresh = load(&document()).expect("loads");

    let merged: Value = serde_json::from_str(
        &fresh.merged_over(&previous).to_json().expect("json"),
    )
    .expect("parses");

    assert_eq!(merged["future_field"], json!("top"));
    assert_eq!(merged["teams"][0]["colour"], json!("blue"));
    assert_eq!(merged["workflows"][0]["owner"], json!("ada"));
}

#[test]
fn a_newer_schema_version_names_the_plugin_upgrade() {
    let mut value = document();
    value["schema_version"] = json!(SCHEMA_VERSION + 1);

    let error = load(&value).expect_err("refused");

    assert!(matches!(error, CatalogueError::NewerSchema { .. }));
    assert!(error.to_string().contains("upgrade"), "{error}");
}

#[test]
fn an_older_schema_version_names_init_discover() {
    let mut value = document();
    value["schema_version"] = json!(0);

    let error = load(&value).expect_err("refused");

    assert!(matches!(error, CatalogueError::OlderSchema { .. }));
    assert!(error.to_string().contains("init discover"), "{error}");
}

#[test]
fn text_that_is_not_a_catalogue_is_unparseable() {
    assert!(matches!(
        Catalogue::parse("not json"),
        Err(CatalogueError::Unparseable { .. })
    ));
}

#[test]
fn a_catalogue_round_trips_through_json() {
    let catalogue = load(&document()).expect("loads");

    let again =
        Catalogue::parse(&catalogue.to_json().expect("json")).expect("loads");

    assert_eq!(again.teams(), catalogue.teams());
}
