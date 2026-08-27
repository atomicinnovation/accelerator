//! `--custom SLUG=VALUE` resolution and coercion against the field cache.

#![allow(clippy::expect_used, clippy::panic)]

use jira_client::custom_fields::coerce;
use jira_client::custom_fields::CustomFieldError;
use serde_json::json;
use serde_json::Value;

fn cache() -> Value {
    json!({
        "site": "acme",
        "fields": [
            {"id": "customfield_1", "key": "cf1", "name": "Story Points",
             "slug": "story-points", "schema": {"type": "number"}},
            {"id": "customfield_2", "key": "cf2", "name": "Team",
             "slug": "team", "schema": {"type": "string"}},
            {"id": "customfield_3", "key": "cf3", "name": "Severity",
             "slug": "severity", "schema": {"type": "option"}},
            {"id": "customfield_4", "key": "cf4", "name": "Owner",
             "slug": "owner", "schema": {"type": "user"}},
            {"id": "customfield_5", "key": "cf5", "name": "Sprint",
             "slug": "sprint", "schema": {"type": "array"}},
            {"id": "customfield_6", "key": "cf6", "name": "Untyped",
             "slug": "untyped"}
        ]
    })
}

fn coerced(entries: &[&str]) -> serde_json::Map<String, Value> {
    let owned: Vec<String> = entries.iter().map(|e| (*e).to_owned()).collect();
    coerce(&cache(), &owned).expect("coercion succeeds")
}

#[test]
fn a_number_field_becomes_a_bare_number() {
    let out = coerced(&["story-points=8"]);
    assert_eq!(out["customfield_1"], json!(8));
}

#[test]
fn a_string_field_becomes_a_json_string() {
    let out = coerced(&["team=Platform"]);
    assert_eq!(out["customfield_2"], json!("Platform"));
}

#[test]
fn an_option_field_becomes_a_value_object() {
    let out = coerced(&["severity=High"]);
    assert_eq!(out["customfield_3"], json!({"value": "High"}));
}

#[test]
fn a_user_field_becomes_an_account_object() {
    let out = coerced(&["owner=5b10a2"]);
    assert_eq!(out["customfield_4"], json!({"accountId": "5b10a2"}));
}

#[test]
fn a_json_escape_bypasses_coercion_for_arrays() {
    let out = coerced(&["sprint=@json:[42]"]);
    assert_eq!(out["customfield_5"], json!([42]));
}

#[test]
fn a_field_resolves_by_name_id_or_key_as_well_as_slug() {
    assert!(coerced(&["Story Points=1"]).contains_key("customfield_1"));
    assert!(coerced(&["customfield_2=x"]).contains_key("customfield_2"));
    assert!(coerced(&["cf3=Low"]).contains_key("customfield_3"));
}

#[test]
fn an_unknown_token_is_an_error() {
    let error =
        coerce(&cache(), &["missing=1".to_owned()]).expect_err("unknown");
    assert!(matches!(error, CustomFieldError::Unknown { .. }), "{error}");
}

#[test]
fn an_entry_without_an_equals_is_malformed() {
    let error =
        coerce(&cache(), &["story-points".to_owned()]).expect_err("malformed");
    assert!(
        matches!(error, CustomFieldError::Malformed { .. }),
        "{error}"
    );
}

#[test]
fn a_non_numeric_value_for_a_number_field_is_rejected() {
    let error = coerce(&cache(), &["story-points=eight".to_owned()])
        .expect_err("bad number");
    assert!(
        matches!(error, CustomFieldError::BadValue { .. }),
        "{error}"
    );
}

#[test]
fn a_scientific_number_is_rejected_like_the_bash_regex() {
    let error =
        coerce(&cache(), &["story-points=1e3".to_owned()]).expect_err("sci");
    assert!(
        matches!(error, CustomFieldError::BadValue { .. }),
        "{error}"
    );
}

#[test]
fn an_untyped_field_needs_a_json_escape() {
    let error =
        coerce(&cache(), &["untyped=x".to_owned()]).expect_err("no type");
    assert!(
        matches!(error, CustomFieldError::BadValue { .. }),
        "{error}"
    );
    let ok = coerce(&cache(), &["untyped=@json:true".to_owned()])
        .expect("escape works");
    assert_eq!(ok["customfield_6"], json!(true));
}

#[test]
fn a_malformed_json_escape_is_rejected() {
    let error = coerce(&cache(), &["sprint=@json:[42".to_owned()])
        .expect_err("bad json");
    assert!(
        matches!(error, CustomFieldError::BadValue { .. }),
        "{error}"
    );
}
