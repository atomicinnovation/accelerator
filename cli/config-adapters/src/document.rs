//! The serde boundary: YAML frontmatter is parsed into a serde-side [`Parsed`]
//! value and mapped into the serde-free `config::Node`, and back for
//! serialization. The mapping preserves each value's type and key order; an
//! integer beyond the `i64` range is preserved as a string.

use std::fmt;

use config::{Mapping, Node, Scalar};
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::frontmatter;

enum Parsed {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Seq(Vec<Parsed>),
    Map(Vec<(String, Parsed)>),
}

/// Parses a whole config file into the typed tree, or empty frontmatter into an
/// empty mapping.
///
/// # Errors
///
/// A detail string when the frontmatter is unterminated or is not valid YAML.
pub fn parse(content: &str) -> Result<Node, String> {
    let split = frontmatter::split(content)?;
    parse_frontmatter(&split.frontmatter)
}

/// Renders `document` as frontmatter, preserving the existing file's body.
///
/// # Errors
///
/// A detail string when the existing file is malformed (so it is never
/// overwritten) or the document cannot be serialized.
pub fn render(
    existing: Option<&str>,
    document: &Node,
) -> Result<String, String> {
    let body = match existing {
        Some(content) => preserved_body(content)?,
        None => String::new(),
    };
    Ok(format!("---\n{}---\n{body}", emit(document)?))
}

fn preserved_body(content: &str) -> Result<String, String> {
    let split = frontmatter::split(content)?;
    parse_frontmatter(&split.frontmatter)?;
    Ok(split.body)
}

fn parse_frontmatter(frontmatter: &str) -> Result<Node, String> {
    if frontmatter.trim().is_empty() {
        return Ok(Node::Mapping(Mapping::new()));
    }
    let parsed: Parsed = serde_saphyr::from_str(frontmatter)
        .map_err(|error| error.to_string())?;
    Ok(to_node(parsed))
}

fn emit(document: &Node) -> Result<String, String> {
    let mut yaml = serde_saphyr::to_string(&to_parsed(document))
        .map_err(|error| error.to_string())?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml)
}

fn to_node(parsed: Parsed) -> Node {
    match parsed {
        Parsed::Null => Node::Scalar(Scalar::Null),
        Parsed::Bool(value) => Node::Scalar(Scalar::Bool(value)),
        Parsed::Int(value) => Node::Scalar(Scalar::Int(value)),
        Parsed::Float(value) => Node::Scalar(Scalar::Float(value)),
        Parsed::Str(value) => Node::Scalar(Scalar::String(value)),
        Parsed::Seq(items) => {
            Node::Sequence(items.into_iter().map(to_node).collect())
        }
        Parsed::Map(entries) => Node::Mapping(
            entries
                .into_iter()
                .map(|(name, value)| (name, to_node(value)))
                .collect(),
        ),
    }
}

fn to_parsed(node: &Node) -> Parsed {
    match node {
        Node::Scalar(Scalar::Bool(value)) => Parsed::Bool(*value),
        Node::Scalar(Scalar::Int(value)) => Parsed::Int(*value),
        Node::Scalar(Scalar::Float(value)) => Parsed::Float(*value),
        Node::Scalar(Scalar::String(value)) => Parsed::Str(value.clone()),
        Node::Scalar(_) => Parsed::Null,
        Node::Sequence(items) => {
            Parsed::Seq(items.iter().map(to_parsed).collect())
        }
        Node::Mapping(mapping) => Parsed::Map(
            mapping
                .entries()
                .iter()
                .map(|(name, value)| (name.clone(), to_parsed(value)))
                .collect(),
        ),
    }
}

struct ParsedVisitor;

impl<'de> Visitor<'de> for ParsedVisitor {
    type Value = Parsed;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any YAML value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Parsed, E> {
        Ok(Parsed::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Parsed, E> {
        Ok(Parsed::Int(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Parsed, E> {
        Ok(i64::try_from(value)
            .map_or_else(|_| Parsed::Str(value.to_string()), Parsed::Int))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Parsed, E> {
        Ok(Parsed::Float(value))
    }

    fn visit_str<E>(self, value: &str) -> Result<Parsed, E> {
        Ok(Parsed::Str(value.to_owned()))
    }

    fn visit_unit<E>(self) -> Result<Parsed, E> {
        Ok(Parsed::Null)
    }

    fn visit_none<E>(self) -> Result<Parsed, E> {
        Ok(Parsed::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<Parsed, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Parsed::Seq(items))
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> Result<Parsed, A::Error> {
        let mut entries = Vec::new();
        while let Some(entry) = map.next_entry::<String, Parsed>()? {
            entries.push(entry);
        }
        Ok(Parsed::Map(entries))
    }
}

impl<'de> Deserialize<'de> for Parsed {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ParsedVisitor)
    }
}

impl Serialize for Parsed {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Int(value) => serializer.serialize_i64(*value),
            Self::Float(value) => serializer.serialize_f64(*value),
            Self::Str(value) => serializer.serialize_str(value),
            Self::Seq(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Self::Map(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (name, value) in entries {
                    map.serialize_entry(name, value)?;
                }
                map.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use config::{Node, Scalar};

    use super::{parse, render};

    fn scalar_at<'a>(node: &'a Node, path: &[&str]) -> Option<&'a Scalar> {
        let mut current = node;
        for segment in path {
            let Node::Mapping(mapping) = current else {
                return None;
            };
            current = mapping.get(segment)?;
        }
        match current {
            Node::Scalar(scalar) => Some(scalar),
            _ => None,
        }
    }

    fn node_at<'a>(node: &'a Node, path: &[&str]) -> Option<&'a Node> {
        let mut current = node;
        for segment in path {
            let Node::Mapping(mapping) = current else {
                return None;
            };
            current = mapping.get(segment)?;
        }
        Some(current)
    }

    #[test]
    fn parses_a_nested_mapping() -> Result<(), String> {
        let node = parse("---\ncore:\n  example: hello\n---\n")?;
        assert_eq!(
            scalar_at(&node, &["core", "example"]),
            Some(&Scalar::String("hello".to_owned()))
        );
        Ok(())
    }

    #[test]
    fn preserves_scalar_types() -> Result<(), String> {
        let node = parse(
            "---\nflag: true\ncount: 7\nratio: 1.5\nnothing:\n\
             items:\n  - a\n  - b\n---\n",
        )?;
        assert_eq!(scalar_at(&node, &["flag"]), Some(&Scalar::Bool(true)));
        assert_eq!(scalar_at(&node, &["count"]), Some(&Scalar::Int(7)));
        assert_eq!(scalar_at(&node, &["ratio"]), Some(&Scalar::Float(1.5)));
        assert_eq!(scalar_at(&node, &["nothing"]), Some(&Scalar::Null));
        let Some(Node::Sequence(items)) = node_at(&node, &["items"]) else {
            return Err("items was not a sequence".to_owned());
        };
        assert_eq!(items.len(), 2);
        Ok(())
    }

    #[test]
    fn an_integer_beyond_i64_becomes_a_string() -> Result<(), String> {
        let node = parse("---\nbig: 10000000000000000000\n---\n")?;
        assert_eq!(
            scalar_at(&node, &["big"]),
            Some(&Scalar::String("10000000000000000000".to_owned()))
        );
        Ok(())
    }

    #[test]
    fn renders_and_reparses_round_trip() -> Result<(), String> {
        let node = parse("---\ncore:\n  example: hello\n---\nbody\n")?;
        let rendered =
            render(Some("---\ncore:\n  example: old\n---\nbody\n"), &node)?;
        assert!(rendered.ends_with("body\n"));
        let reparsed = parse(&rendered)?;
        assert_eq!(
            scalar_at(&reparsed, &["core", "example"]),
            Some(&Scalar::String("hello".to_owned()))
        );
        Ok(())
    }
}
