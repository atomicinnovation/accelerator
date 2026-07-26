//! Serde bridge for the serde-free `corpus::DocTypeKey`.
//!
//! The API wire form is the kebab-case token; `corpus::DocTypeKey` carries no
//! serde derive by design, so struct fields serialise it through this module
//! via `#[serde(with = "crate::doc_type_serde")]`, mapping through
//! `wire_str`/`from_wire_str` rather than re-deriving serde on the shared type.

use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S: Serializer>(
    kind: &corpus::DocTypeKey,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(kind.wire_str())
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<corpus::DocTypeKey, D::Error> {
    let token = String::deserialize(deserializer)?;
    corpus::DocTypeKey::from_wire_str(&token).ok_or_else(|| {
        serde::de::Error::custom(format!("unknown doc type: {token}"))
    })
}
