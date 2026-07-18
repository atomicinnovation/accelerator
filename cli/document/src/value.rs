//! The serde-carrying frontmatter value tree.
//!
//! Structurally 1:1 with `config::Node` / `corpus::FrontmatterValue`; this is
//! the format layer's serde boundary. An integer beyond the `i64` range is
//! preserved as a string, key order is preserved, and the hand-written
//! `Serialize`/`Deserialize` map every YAML value onto these variants.

use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

/// A value in a parsed frontmatter tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Yaml {
    Scalar(Scalar),
    Sequence(Vec<Yaml>),
    Mapping(Mapping),
}

/// A leaf value.
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    Null,
}

/// An insertion-ordered mapping.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mapping(Vec<(String, Yaml)>);

impl Mapping {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Yaml> {
        self.0
            .iter()
            .find(|(existing, _)| existing == key)
            .map(|(_, value)| value)
    }

    pub fn push(&mut self, key: String, value: Yaml) {
        self.0.push((key, value));
    }

    #[must_use]
    pub fn entries(&self) -> &[(String, Yaml)] {
        &self.0
    }
}

impl IntoIterator for Mapping {
    type Item = (String, Yaml);
    type IntoIter = std::vec::IntoIter<(String, Yaml)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(String, Yaml)> for Mapping {
    fn from_iter<I: IntoIterator<Item = (String, Yaml)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

struct YamlVisitor;

impl<'de> Visitor<'de> for YamlVisitor {
    type Value = Yaml;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any YAML value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Yaml, E> {
        Ok(Yaml::Scalar(Scalar::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Yaml, E> {
        Ok(Yaml::Scalar(Scalar::Int(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Yaml, E> {
        Ok(i64::try_from(value).map_or_else(
            |_| Yaml::Scalar(Scalar::String(value.to_string())),
            |int| Yaml::Scalar(Scalar::Int(int)),
        ))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Yaml, E> {
        Ok(Yaml::Scalar(Scalar::Float(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Yaml, E> {
        Ok(Yaml::Scalar(Scalar::String(value.to_owned())))
    }

    fn visit_unit<E>(self) -> Result<Yaml, E> {
        Ok(Yaml::Scalar(Scalar::Null))
    }

    fn visit_none<E>(self) -> Result<Yaml, E> {
        Ok(Yaml::Scalar(Scalar::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<Yaml, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Yaml::Sequence(items))
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> Result<Yaml, A::Error> {
        let mut mapping = Mapping::new();
        while let Some((key, value)) = map.next_entry::<String, Yaml>()? {
            mapping.push(key, value);
        }
        Ok(Yaml::Mapping(mapping))
    }
}

impl<'de> Deserialize<'de> for Yaml {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer.deserialize_any(YamlVisitor)
    }
}

impl Serialize for Yaml {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Self::Scalar(Scalar::Null) => serializer.serialize_unit(),
            Self::Scalar(Scalar::Bool(value)) => {
                serializer.serialize_bool(*value)
            }
            Self::Scalar(Scalar::Int(value)) => {
                serializer.serialize_i64(*value)
            }
            Self::Scalar(Scalar::Float(value)) => {
                serializer.serialize_f64(*value)
            }
            Self::Scalar(Scalar::String(value)) => {
                serializer.serialize_str(value)
            }
            Self::Sequence(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Self::Mapping(mapping) => {
                let entries = mapping.entries();
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}
