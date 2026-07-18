//! Rendering a [`Yaml`] frontmatter tree back into a document, preserving an
//! existing document's body.

use crate::error::DocumentError;
use crate::fence;
use crate::parse::parse_frontmatter;
use crate::value::Yaml;

/// Renders `frontmatter` as a document, preserving `existing`'s body.
///
/// `existing`'s frontmatter is re-parsed (not merely fence-split): a
/// fence-valid but YAML-invalid existing document errors here rather than being
/// overwritten.
///
/// # Errors
///
/// [`DocumentError`] when the existing document's frontmatter is malformed or
/// the frontmatter cannot be serialised.
pub fn render(
    existing: Option<&str>,
    frontmatter: &Yaml,
) -> Result<String, DocumentError> {
    let body = match existing {
        Some(content) => preserved_body(content)?,
        None => String::new(),
    };
    Ok(format!("---\n{}---\n{body}", emit(frontmatter)?))
}

fn preserved_body(content: &str) -> Result<String, DocumentError> {
    let split = fence::split(content)?;
    parse_frontmatter(&split.frontmatter)?;
    Ok(split.body)
}

fn emit(frontmatter: &Yaml) -> Result<String, DocumentError> {
    let mut yaml = serde_saphyr::to_string(frontmatter)
        .map_err(|error| DocumentError::Emit(error.to_string()))?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml)
}
