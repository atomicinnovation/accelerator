//! Parsing a document's frontmatter into the [`Yaml`] value tree.

use crate::error::DocumentError;
use crate::fence;
use crate::value::{Mapping, Yaml};

/// Parses `content`'s frontmatter into a [`Yaml`] tree; empty frontmatter
/// yields an empty mapping.
///
/// # Errors
///
/// [`DocumentError::Unterminated`] when the frontmatter is unclosed, or
/// [`DocumentError::InvalidYaml`] when it is not valid YAML.
pub fn parse(content: &str) -> Result<Yaml, DocumentError> {
    let split = fence::split(content)?;
    parse_frontmatter(&split.frontmatter)
}

pub(crate) fn parse_frontmatter(
    frontmatter: &str,
) -> Result<Yaml, DocumentError> {
    if frontmatter.trim().is_empty() {
        return Ok(Yaml::Mapping(Mapping::new()));
    }
    serde_saphyr::from_str(frontmatter)
        .map_err(|error| DocumentError::InvalidYaml(error.to_string()))
}
