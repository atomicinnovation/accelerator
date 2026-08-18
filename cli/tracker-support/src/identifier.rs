//! The tracker-agnostic safety check on an identifier that will be written
//! back into a work item's unquoted YAML frontmatter.
//!
//! Scoped narrowly to what actually breaks an unquoted YAML scalar: `/`, `#`
//! and `@` are permitted mid-token, because GitHub (`owner/repo#42`) and
//! Trello identifiers legitimately carry them. Every character
//! `char::is_control` reports is refused, covering the C1 block as well as C0
//! and DEL — refusing more can only keep a corrupting identifier out.

use std::error::Error;
use std::fmt;

/// Why an identifier cannot be written back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierRefusal {
    Empty,
    ControlCharacter,
    DocumentSeparator,
    CommentTrigger,
}

impl fmt::Display for IdentifierRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match *self {
            Self::Empty => "the identifier is empty",
            Self::ControlCharacter => {
                "the identifier carries a control character"
            }
            Self::DocumentSeparator => {
                "the identifier starts with a YAML document separator"
            }
            Self::CommentTrigger => {
                "the identifier starts with a YAML comment marker"
            }
        };
        formatter.write_str(reason)
    }
}

impl Error for IdentifierRefusal {}

/// Accepts an identifier safe to write into unquoted YAML frontmatter.
///
/// # Errors
///
/// [`IdentifierRefusal`] naming the property that failed.
pub fn identifier_is_safe(candidate: &str) -> Result<(), IdentifierRefusal> {
    if candidate.is_empty() {
        return Err(IdentifierRefusal::Empty);
    }
    if candidate.chars().any(char::is_control) {
        return Err(IdentifierRefusal::ControlCharacter);
    }
    if candidate.starts_with("---") {
        return Err(IdentifierRefusal::DocumentSeparator);
    }
    if candidate.trim_start().starts_with('#') {
        return Err(IdentifierRefusal::CommentTrigger);
    }
    Ok(())
}
