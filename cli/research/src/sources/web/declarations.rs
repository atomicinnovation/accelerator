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
    pub(crate) fn has_any_type_of(&self, types: &[&str]) -> bool {
        self.types.iter().any(|kind| types.contains(&kind.as_str()))
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
