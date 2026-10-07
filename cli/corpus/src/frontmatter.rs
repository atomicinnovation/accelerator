//! A document's frontmatter as the corpus sees it, and the port that reads
//! it from text.

use std::fmt::Display;
use std::fmt::Formatter;

use crate::value::FrontmatterValue;
use crate::value::Mapping;

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

/// Why a document's frontmatter could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterError(pub String);

impl Display for FrontmatterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for FrontmatterError {}

/// Reads a document's frontmatter.
pub trait FrontmatterParser {
    /// Classifies the frontmatter and splits off the body. A non-mapping root
    /// is `Malformed`, a null or empty root is an empty `Parsed` mapping, and
    /// unfenced content is `Absent`.
    fn classify(&self, raw: &[u8]) -> ParsedDocument;

    /// The frontmatter's value, whatever its root. Unfenced or empty
    /// frontmatter is an empty mapping.
    ///
    /// # Errors
    ///
    /// A [`FrontmatterError`] for an unterminated fence, invalid YAML, or a
    /// tagged node.
    fn parse_value(
        &self,
        content: &str,
    ) -> Result<FrontmatterValue, FrontmatterError>;

    /// The raw text between the fences, whatever its root; empty for
    /// unfenced content.
    ///
    /// # Errors
    ///
    /// A [`FrontmatterError`] for an unterminated fence.
    fn split_frontmatter(
        &self,
        content: &str,
    ) -> Result<String, FrontmatterError>;
}
