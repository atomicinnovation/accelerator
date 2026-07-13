//! Translating a parsed `document::Yaml` into the `corpus::FrontmatterValue`
//! domain tree, and the per-document frontmatter-state classification.

use corpus::FrontmatterValue;
use corpus::Mapping;
use corpus::Scalar;
use document::Scalar as YamlScalar;
use document::Yaml;

/// The frontmatter outcome for a document: parsed to a root mapping, absent, or
/// malformed.
#[derive(Debug, Clone, PartialEq)]
pub enum FrontmatterState {
    Parsed(Mapping),
    Absent,
    Malformed,
}

/// A classified document: its frontmatter state and its body.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub state: FrontmatterState,
    pub body: String,
}

/// Classifies a document's frontmatter and splits off its body. A non-mapping
/// root (a sequence or a non-null scalar) is `Malformed`; a null or empty root
/// is a `Parsed` empty mapping.
#[must_use]
pub fn parse(raw: &[u8]) -> ParsedDocument {
    std::str::from_utf8(raw)
        .map_or_else(|_| classify(&String::from_utf8_lossy(raw)), classify)
}

fn classify(content: &str) -> ParsedDocument {
    match document::fence_offsets(content.as_bytes()) {
        Ok(None) => ParsedDocument {
            state: FrontmatterState::Absent,
            body: content.to_owned(),
        },
        Err(_) => ParsedDocument {
            state: FrontmatterState::Malformed,
            body: String::new(),
        },
        Ok(Some((_, body_start))) => {
            let body = content[body_start..].to_owned();
            ParsedDocument {
                state: root_state(content),
                body,
            }
        }
    }
}

fn root_state(content: &str) -> FrontmatterState {
    match document::parse(content) {
        Ok(Yaml::Mapping(mapping)) => {
            FrontmatterState::Parsed(to_mapping(mapping))
        }
        Ok(Yaml::Scalar(YamlScalar::Null)) => {
            FrontmatterState::Parsed(Mapping::new())
        }
        Ok(_) | Err(_) => FrontmatterState::Malformed,
    }
}

/// Maps a parsed `document::Yaml` tree into the domain `FrontmatterValue`.
#[must_use]
pub fn to_value(value: Yaml) -> FrontmatterValue {
    match value {
        Yaml::Scalar(scalar) => FrontmatterValue::Scalar(to_scalar(scalar)),
        Yaml::Sequence(items) => FrontmatterValue::Sequence(
            items.into_iter().map(to_value).collect(),
        ),
        Yaml::Mapping(mapping) => {
            FrontmatterValue::Mapping(to_mapping(mapping))
        }
    }
}

fn to_mapping(mapping: document::Mapping) -> Mapping {
    mapping
        .into_iter()
        .map(|(key, value)| (key, to_value(value)))
        .collect()
}

fn to_scalar(scalar: YamlScalar) -> Scalar {
    match scalar {
        YamlScalar::String(value) => Scalar::String(value),
        YamlScalar::Bool(value) => Scalar::Bool(value),
        YamlScalar::Int(value) => Scalar::Int(value),
        YamlScalar::Float(value) => Scalar::Float(value),
        YamlScalar::Null => Scalar::Null,
    }
}

#[cfg(test)]
mod tests {
    use corpus::{FrontmatterValue, Scalar};

    use super::{parse, FrontmatterState};

    fn parsed_get(state: &FrontmatterState, key: &str) -> Option<Scalar> {
        let FrontmatterState::Parsed(mapping) = state else {
            return None;
        };
        match mapping.get(key)? {
            FrontmatterValue::Scalar(scalar) => Some(scalar.clone()),
            _ => None,
        }
    }

    #[test]
    fn parses_a_mapping_root_and_splits_the_body() {
        let document = parse(b"---\ntitle: Foo\nstatus: done\n---\n# Body\n");
        assert_eq!(
            parsed_get(&document.state, "title"),
            Some(Scalar::String("Foo".to_owned()))
        );
        assert!(document.body.starts_with("# Body"));
    }

    #[test]
    fn absent_when_no_leading_fence() {
        let document = parse(b"# Notes\n\ncontent\n");
        assert_eq!(document.state, FrontmatterState::Absent);
    }

    #[test]
    fn malformed_when_no_closing_fence() {
        let document = parse(b"---\ntitle: foo\n");
        assert_eq!(document.state, FrontmatterState::Malformed);
    }

    #[test]
    fn a_null_or_empty_root_is_an_empty_parsed_mapping() {
        for raw in [b"---\n---\nbody\n".as_slice(), b"---\nnull\n---\nbody\n"] {
            let document = parse(raw);
            assert!(matches!(
                document.state,
                FrontmatterState::Parsed(ref mapping) if mapping.entries().is_empty()
            ));
        }
    }

    #[test]
    fn a_sequence_root_is_malformed() {
        let document = parse(b"---\n- a\n- b\n---\nbody\n");
        assert_eq!(document.state, FrontmatterState::Malformed);
    }

    #[test]
    fn a_scalar_root_is_malformed() {
        let document = parse(b"---\njust a string\n---\nbody\n");
        assert_eq!(document.state, FrontmatterState::Malformed);
    }

    #[test]
    fn a_tagged_node_is_malformed() {
        // Fail closed on every shape a tag can take. Without the guard
        // serde-saphyr resolves the tag away and the node parses as its base
        // value, silently losing what the document actually said.
        for raw in [
            b"---\nkey: !custom value\n---\nbody\n".as_slice(),
            b"---\nkey: !!str 123\n---\nbody\n",
            b"---\nkey: !!int 7\n---\nbody\n",
            b"---\nouter:\n  inner: !custom v\n---\nbody\n",
            b"---\nkey:\n  - !custom v\n---\nbody\n",
            b"---\n!custom {a: 1}\n---\nbody\n",
        ] {
            let document = parse(raw);
            assert_eq!(
                document.state,
                FrontmatterState::Malformed,
                "expected Malformed for {}",
                String::from_utf8_lossy(raw)
            );
        }
    }

    #[test]
    fn a_tag_inside_a_quoted_scalar_still_parses() {
        let document = parse(b"---\nnote: \"see !!important\"\n---\nbody\n");
        assert!(matches!(
            document.state,
            FrontmatterState::Parsed(ref mapping)
                if mapping.get("note")
                    == Some(&FrontmatterValue::Scalar(Scalar::String(
                        "see !!important".to_owned()
                    )))
        ));
    }
}
