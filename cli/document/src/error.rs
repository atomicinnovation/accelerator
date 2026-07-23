//! The document-protocol error taxonomy.

use std::fmt::Display;
use std::fmt::Formatter;

/// A failure splitting, parsing, or rendering a markdown-frontmatter document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentError {
    /// A frontmatter block was opened but never closed.
    Unterminated,
    /// The frontmatter is not valid YAML.
    InvalidYaml(String),
    /// A frontmatter node carries an explicit YAML tag.
    ///
    /// Tags are rejected rather than resolved: serde-saphyr would hand back the
    /// tag's base value, silently discarding the tag, so a document could not
    /// round-trip through the value tree unchanged.
    Tagged(String),
    /// A frontmatter tree could not be serialised back to YAML.
    Emit(String),
}

impl Display for DocumentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unterminated => {
                write!(formatter, "unterminated frontmatter block")
            }
            Self::InvalidYaml(detail) => {
                write!(formatter, "invalid frontmatter YAML: {detail}")
            }
            Self::Tagged(tag) => {
                write!(formatter, "frontmatter carries a YAML tag: {tag}")
            }
            Self::Emit(detail) => {
                write!(formatter, "could not render frontmatter: {detail}")
            }
        }
    }
}

impl std::error::Error for DocumentError {}
