//! What a page declares about itself, as the decoder hands it over: its
//! meta tags and its JSON-LD nodes, each in document order.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageDeclarations {
    pub meta_tags: Vec<MetaTag>,
    /// Every top-level node, each element of a top-level array and each
    /// element of any `@graph`.
    pub linked_data: Vec<LinkedDataNode>,
}

/// A `meta` tag's `name`, else its `property`, and its `content`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaTag {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkedDataNode {
    pub types: Vec<String>,
    pub properties: Vec<(String, LinkedDataValue)>,
}

/// JSON numbers and booleans arrive as text, and `null` not at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkedDataValue {
    Text(String),
    Node(LinkedDataNode),
    List(Vec<LinkedDataValue>),
}

pub trait PageDecoder {
    fn declarations(&self, html: &[u8]) -> PageDeclarations;
}

impl LinkedDataNode {
    /// Where this node's most specific type falls in `ranked`, a type named
    /// by its schema.org IRI or `schema:` prefix counting as its bare name.
    pub(crate) fn type_rank(&self, ranked: &[&str]) -> Option<usize> {
        self.types
            .iter()
            .filter_map(|kind| {
                ranked.iter().position(|name| *name == schema_name(kind))
            })
            .min()
    }

    /// The node or nodes this one declares itself to be about.
    pub(crate) fn main_entities(&self) -> Vec<&Self> {
        match self.property("mainEntity") {
            Some(LinkedDataValue::Node(node)) => vec![node],
            Some(LinkedDataValue::List(values)) => values
                .iter()
                .filter_map(|value| match value {
                    LinkedDataValue::Node(node) => Some(node),
                    LinkedDataValue::Text(_) | LinkedDataValue::List(_) => None,
                })
                .collect(),
            Some(LinkedDataValue::Text(_)) | None => Vec::new(),
        }
    }

    pub(crate) fn property(&self, name: &str) -> Option<&LinkedDataValue> {
        self.properties
            .iter()
            .find(|(present, _)| present == name)
            .map(|(_, value)| value)
    }

    pub(crate) fn text_of(&self, name: &str) -> Option<&str> {
        match self.property(name)? {
            LinkedDataValue::Text(text) => Some(text),
            LinkedDataValue::Node(_) | LinkedDataValue::List(_) => None,
        }
    }
}

fn schema_name(kind: &str) -> &str {
    ["https://schema.org/", "http://schema.org/", "schema:"]
        .iter()
        .find_map(|prefix| kind.strip_prefix(prefix))
        .unwrap_or(kind)
}
