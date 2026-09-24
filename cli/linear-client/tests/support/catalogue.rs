//! `catalogue.json` records and entries as the suites seed them.

#![allow(dead_code, clippy::expect_used)]

use linear_client::catalogue::Catalogue;
use linear_client::resolution::ResolverSet;
use serde_json::{json, Value};

pub const ARCHIVED: &str = "2026-01-01T00:00:00Z";

#[must_use]
pub fn state(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "type": "started", "position": 1,
            "archivedAt": null })
}

#[must_use]
pub fn label(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "archivedAt": null })
}

#[must_use]
pub fn project(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "archivedAt": null })
}

#[must_use]
pub fn member(id: &str, name: &str, email: &str) -> Value {
    json!({ "id": id, "name": name, "displayName": name,
            "email": email, "active": true })
}

/// A team entry carrying exactly the sections `sections` names.
#[must_use]
pub fn entry(id: &str, key: &str, sections: &Value) -> Value {
    let mut entry = json!({ "id": id, "key": key, "name": key });
    for (section, records) in sections.as_object().expect("an object") {
        entry[section] = records.clone();
    }
    entry
}

/// An entry carrying all four sections, the empty ones as `[]`.
#[must_use]
pub fn complete_entry(id: &str, key: &str, sections: &Value) -> Value {
    let mut all = json!({
        "states": [state(&format!("{id}-todo"), "Todo")],
        "labels": [],
        "members": [],
        "projects": []
    });
    for (section, records) in sections.as_object().expect("an object") {
        all[section] = records.clone();
    }
    entry(id, key, &all)
}

#[must_use]
pub fn resolvers(catalogue: &Value) -> ResolverSet {
    Catalogue::from_text(&catalogue.to_string()).resolver_set()
}
