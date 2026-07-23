//! The serde-free frontmatter value tree.

/// A value in a parsed frontmatter tree.
#[derive(Debug, Clone, PartialEq)]
pub enum FrontmatterValue {
    Scalar(Scalar),
    Sequence(Vec<FrontmatterValue>),
    Mapping(Mapping),
}

/// A leaf value, typed so a rewrite keeps a sibling's kind intact.
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
pub struct Mapping(Vec<(String, FrontmatterValue)>);

impl Mapping {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&FrontmatterValue> {
        self.0
            .iter()
            .find(|(existing, _)| existing == key)
            .map(|(_, value)| value)
    }

    pub fn push(&mut self, key: String, value: FrontmatterValue) {
        self.0.push((key, value));
    }

    #[must_use]
    pub fn entries(&self) -> &[(String, FrontmatterValue)] {
        &self.0
    }
}

impl FromIterator<(String, FrontmatterValue)> for Mapping {
    fn from_iter<I: IntoIterator<Item = (String, FrontmatterValue)>>(
        iter: I,
    ) -> Self {
        Self(iter.into_iter().collect())
    }
}
