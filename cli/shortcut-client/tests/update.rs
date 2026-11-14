use serde_json::json;
use shortcut_client::update::UpdateFields;

#[test]
fn only_the_given_fields_are_sent() {
    let fields = UpdateFields {
        name: Some("New title".to_owned()),
        ..UpdateFields::default()
    };

    assert_eq!(fields.body(), json!({"name": "New title"}));
}

#[test]
fn an_empty_description_is_sent_as_given_not_dropped() {
    let fields = UpdateFields {
        description: Some(String::new()),
        ..UpdateFields::default()
    };

    assert_eq!(fields.body(), json!({"description": ""}));
}

#[test]
fn every_field_lands_in_one_body() {
    let fields = UpdateFields {
        name: Some("t".to_owned()),
        description: Some("d".to_owned()),
        workflow_state_id: Some(7),
        owner_ids: Some(vec!["m-1".to_owned()]),
        estimate: Some(3),
    };

    assert_eq!(
        fields.body(),
        json!({"name": "t", "description": "d", "workflow_state_id": 7,
               "owner_ids": ["m-1"], "estimate": 3})
    );
}

#[test]
fn no_fields_is_empty() {
    assert!(UpdateFields::default().is_empty());
    assert_eq!(UpdateFields::default().body(), json!({}));
}
