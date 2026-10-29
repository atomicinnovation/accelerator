//! A web page's HTML, decoded into the meta tags and JSON-LD nodes it
//! declares.
//!
//! Undecodable bytes become replacement characters and an unparseable
//! JSON-LD script is skipped, so a page always yields what it can.

use std::borrow::Cow;

use html5gum::DefaultEmitter;
use html5gum::StartTag;
use html5gum::Token;
use html5gum::Tokenizer;
use research::sources::web::declarations::LinkedDataNode;
use research::sources::web::declarations::LinkedDataValue;
use research::sources::web::declarations::MetaTag;
use research::sources::web::declarations::PageDeclarations;
use research::sources::web::declarations::PageDecoder;
use serde_json::Map;
use serde_json::Value;

pub struct WebHtmlDecoder;

impl PageDecoder for WebHtmlDecoder {
    fn declarations(&self, html: &[u8]) -> PageDeclarations {
        let html = String::from_utf8_lossy(html);
        let mut declarations = PageDeclarations::default();
        let mut open_script: Option<String> = None;
        for token in tokens(&html) {
            match token {
                Token::StartTag(tag) if tag.name == b"meta" => {
                    declarations.meta_tags.extend(meta_tag(&tag));
                }
                Token::StartTag(tag)
                    if tag.name == b"script" && is_linked_data(&tag) =>
                {
                    open_script = Some(String::new());
                }
                Token::String(text) => {
                    if let Some(script) = open_script.as_mut() {
                        script.push_str(&lossy(&text));
                    }
                }
                Token::EndTag(tag) if tag.name == b"script" => {
                    if let Some(script) = open_script.take() {
                        collect_nodes(&script, &mut declarations.linked_data);
                    }
                }
                _ => {}
            }
        }
        declarations
    }
}

/// Without a tree builder the tokeniser would read a script's body as
/// markup, so a `<` inside a JSON-LD string would end the script early.
fn tokens(html: &str) -> impl Iterator<Item = Token> + '_ {
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    Tokenizer::new_with_emitter(html, emitter).flatten()
}

fn meta_tag(tag: &StartTag<()>) -> Option<MetaTag> {
    let name = attribute(tag, "name").or_else(|| attribute(tag, "property"))?;
    let content = attribute(tag, "content")?;
    Some(MetaTag { name, content })
}

fn is_linked_data(tag: &StartTag<()>) -> bool {
    attribute(tag, "type").is_some_and(|media_type| {
        media_type
            .trim()
            .eq_ignore_ascii_case("application/ld+json")
    })
}

fn attribute(tag: &StartTag<()>, name: &str) -> Option<String> {
    tag.attributes
        .get(name.as_bytes())
        .map(|value| lossy(value).into_owned())
}

fn lossy(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn collect_nodes(script: &str, nodes: &mut Vec<LinkedDataNode>) {
    if let Ok(document) = serde_json::from_str::<Value>(script) {
        collect_document_nodes(&document, nodes);
    }
}

fn collect_document_nodes(value: &Value, nodes: &mut Vec<LinkedDataNode>) {
    match value {
        Value::Object(object) => {
            nodes.push(node(object));
            if let Some(graph) = object.get("@graph") {
                collect_document_nodes(graph, nodes);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_document_nodes(value, nodes);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn node(object: &Map<String, Value>) -> LinkedDataNode {
    LinkedDataNode {
        types: object.get("@type").map(types).unwrap_or_default(),
        properties: object
            .iter()
            .filter(|(name, _)| !matches!(name.as_str(), "@type" | "@graph"))
            .filter_map(|(name, value)| {
                Some((name.clone(), linked_value(value)?))
            })
            .collect(),
    }
}

fn types(value: &Value) -> Vec<String> {
    match value {
        Value::String(kind) => vec![kind.clone()],
        Value::Array(kinds) => kinds
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Object(_) => {
            Vec::new()
        }
    }
}

fn linked_value(value: &Value) -> Option<LinkedDataValue> {
    match value {
        Value::Null => None,
        Value::Bool(flag) => Some(LinkedDataValue::Text(flag.to_string())),
        Value::Number(number) => {
            Some(LinkedDataValue::Text(number.to_string()))
        }
        Value::String(text) => Some(LinkedDataValue::Text(text.clone())),
        Value::Object(object) => Some(LinkedDataValue::Node(node(object))),
        Value::Array(values) => Some(LinkedDataValue::List(
            values.iter().filter_map(linked_value).collect(),
        )),
    }
}
