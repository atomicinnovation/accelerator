//! A document's frontmatter read as YAML, for the string fields a reader
//! needs decoded rather than as the renderer escaped them.

use document::DocumentError;
use document::Scalar;
use document::Yaml;

pub struct FrontmatterStrings(Yaml);

impl FrontmatterStrings {
    /// # Errors
    ///
    /// [`DocumentError`] when the frontmatter is unterminated or not valid
    /// YAML.
    pub fn parse(content: &str) -> Result<Self, DocumentError> {
        document::parse(content).map(Self)
    }

    /// `key`'s value, or `None` when it is absent or not a string.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        match &self.0 {
            Yaml::Mapping(mapping) => match mapping.get(key) {
                Some(Yaml::Scalar(Scalar::String(value))) => Some(value),
                _ => None,
            },
            Yaml::Scalar(_) | Yaml::Sequence(_) => None,
        }
    }
}
