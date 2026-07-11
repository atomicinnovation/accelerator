//! The boundary between the `document` format crate and `config`'s domain
//! `Node` tree. Parsing and rendering delegate to `document`; this module only
//! maps between the two structurally-identical value shapes.

use ::document::{Scalar as YamlScalar, Yaml};
use config::{Node, Scalar};

/// Parses a whole config file into the typed tree, or empty frontmatter into an
/// empty mapping.
///
/// # Errors
///
/// A detail string when the frontmatter is unterminated or is not valid YAML.
pub fn parse(content: &str) -> Result<Node, String> {
    ::document::parse(content)
        .map(to_node)
        .map_err(|error| error.to_string())
}

/// Renders `node` as frontmatter, preserving the existing file's body.
///
/// # Errors
///
/// A detail string when the existing file is malformed (so it is never
/// overwritten) or the document cannot be serialized.
pub fn render(existing: Option<&str>, node: &Node) -> Result<String, String> {
    ::document::render(existing, &to_yaml(node))
        .map_err(|error| error.to_string())
}

fn to_node(value: Yaml) -> Node {
    match value {
        Yaml::Scalar(scalar) => Node::Scalar(to_node_scalar(scalar)),
        Yaml::Sequence(items) => {
            Node::Sequence(items.into_iter().map(to_node).collect())
        }
        Yaml::Mapping(mapping) => Node::Mapping(
            mapping
                .into_iter()
                .map(|(key, value)| (key, to_node(value)))
                .collect(),
        ),
    }
}

fn to_node_scalar(scalar: YamlScalar) -> Scalar {
    match scalar {
        YamlScalar::String(value) => Scalar::String(value),
        YamlScalar::Bool(value) => Scalar::Bool(value),
        YamlScalar::Int(value) => Scalar::Int(value),
        YamlScalar::Float(value) => Scalar::Float(value),
        YamlScalar::Null => Scalar::Null,
    }
}

fn to_yaml(node: &Node) -> Yaml {
    match node {
        Node::Scalar(scalar) => Yaml::Scalar(to_yaml_scalar(scalar)),
        Node::Sequence(items) => {
            Yaml::Sequence(items.iter().map(to_yaml).collect())
        }
        Node::Mapping(mapping) => Yaml::Mapping(
            mapping
                .entries()
                .iter()
                .map(|(key, value)| (key.clone(), to_yaml(value)))
                .collect(),
        ),
    }
}

fn to_yaml_scalar(scalar: &Scalar) -> YamlScalar {
    match scalar {
        Scalar::String(value) => YamlScalar::String(value.clone()),
        Scalar::Bool(value) => YamlScalar::Bool(*value),
        Scalar::Int(value) => YamlScalar::Int(*value),
        Scalar::Float(value) => YamlScalar::Float(*value),
        Scalar::Null => YamlScalar::Null,
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
