//! Both conversion directions between Atlassian Document Format and a
//! bespoke Markdown subset.
//!
//! The dialect is not `CommonMark` and not ADF-complete: no text escaping, a
//! fixed `code → em → strong → link` mark pipeline, `attrs.order` always 1 on
//! the way in, seeded `localId`s, and silent drops for `strike`, `underline`
//! and a `listItem`'s second and later children. No third-party crate
//! reproduces it byte for byte, so it is hand-built.
//!
//! The two directions accept different languages, deliberately. Rendering
//! accepts a strictly larger one and degrades unknown nodes to a placeholder;
//! assembly hard-rejects three constructs. `tests/fixtures/adf-fidelity-quirks.txt`
//! records every behaviour that is knowingly wrong but faithful.

pub mod assemble;
pub mod render;
pub mod tokenise;

use std::error::Error;
use std::fmt;

use serde_json::Value;

/// Why a conversion could not be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdfError {
    RootNotDoc { found: String },
    HeadingWithoutLevel,
    ListWithoutContent { node: String },
    UnsupportedBlockquote,
    UnsupportedTable,
    UnsupportedNestedList,
    BadInput,
}

impl AdfError {
    /// The exit code reported for this condition.
    #[must_use]
    pub const fn code(&self) -> u16 {
        match *self {
            Self::RootNotDoc { .. }
            | Self::HeadingWithoutLevel
            | Self::ListWithoutContent { .. } => 40,
            Self::UnsupportedBlockquote
            | Self::UnsupportedTable
            | Self::UnsupportedNestedList => 41,
            Self::BadInput => 42,
        }
    }
}

impl fmt::Display for AdfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootNotDoc { found } => write!(
                formatter,
                "E_BAD_JSON: input is not an ADF document (missing type=doc, \
                 found {found})"
            ),
            Self::HeadingWithoutLevel => write!(
                formatter,
                "E_BAD_JSON: a heading carries no numeric attrs.level"
            ),
            Self::ListWithoutContent { node } => write!(
                formatter,
                "E_BAD_JSON: a {node} carries no content array"
            ),
            Self::UnsupportedBlockquote => write!(
                formatter,
                "E_ADF_UNSUPPORTED_BLOCKQUOTE: blockquote is not supported"
            ),
            Self::UnsupportedTable => write!(
                formatter,
                "E_ADF_UNSUPPORTED_TABLE: pipe tables are not supported"
            ),
            Self::UnsupportedNestedList => write!(
                formatter,
                "E_ADF_UNSUPPORTED_NESTED_LIST: nested lists are not supported"
            ),
            Self::BadInput => write!(
                formatter,
                "E_ADF_BAD_INPUT: input contains control byte \\x1e or \\x1f"
            ),
        }
    }
}

impl Error for AdfError {}

/// The whole markdown-to-ADF pipeline: tokenise, then assemble.
///
/// `seed` selects the localid form: `Some` gives the deterministic
/// `00000000-0000-4000-8000-00000000000N` form, `None` the bare counter. It is
/// a parameter rather than an environment read so a test needs no process
/// state.
///
/// # Errors
///
/// [`AdfError`] for the three rejected constructs and for a control byte.
pub fn markdown_to_document(
    markdown: &str,
    seed: Option<&str>,
) -> Result<Value, AdfError> {
    let tokenised = tokenise::tokenise(markdown)?;
    Ok(assemble::to_document(&tokenised.tokens, seed))
}

/// The whole ADF-to-markdown direction.
///
/// # Errors
///
/// [`AdfError`] when the root is not a `doc`, a heading carries no level, or a
/// list carries no content.
pub fn document_to_markdown(document: &Value) -> Result<String, AdfError> {
    render::to_markdown(document)
}
