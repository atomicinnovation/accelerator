//! Both conversion directions between Atlassian Document Format and a
//! bespoke Markdown subset.
//!
//! The dialect is not `CommonMark` and not ADF-complete: a fixed
//! `code → em → strong → strike → link` mark pipeline, seeded `localId`s, and
//! silent drops for `underline` and every other mark without a Markdown form.
//! No third-party crate reproduces it byte for byte, so it is hand-built.
//!
//! Text that the tokeniser would read as syntax is backslash-escaped on the
//! way out, so every body rendered strictly is a fixed point of the round
//! trip. Rendering accepts a strictly larger language than assembly produces:
//! a node with no Markdown form degrades to a placeholder, or refuses the
//! render when it is strict. `tests/fixtures/adf-fidelity-quirks.txt` records
//! every behaviour that is knowingly wrong but faithful.

mod assemble;
mod render;
mod syntax;
mod tokenise;

use std::error::Error;
use std::fmt;

use serde_json::Value;

use render::NoMarkdownForm;

/// Why a conversion could not be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdfError {
    RootNotDoc { found: String },
    HeadingWithoutLevel,
    ListWithoutContent { node: String },
    Unrepresentable { node: String },
    BadInput,
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
            Self::Unrepresentable { node } => write!(
                formatter,
                "E_ADF_UNREPRESENTABLE: a {node} has no Markdown form"
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
/// [`AdfError::BadInput`] for a control byte.
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
    render::to_markdown(document, NoMarkdownForm::Placeholder)
}

/// The ADF-to-markdown direction that sync relies on: it refuses, rather than
/// degrades, a node with no Markdown form, so what it renders reads back as
/// the same Markdown.
///
/// # Errors
///
/// [`AdfError::Unrepresentable`] at the first node with no Markdown form,
/// including a heading above level six, and every error
/// [`document_to_markdown`] raises.
pub fn document_to_markdown_strict(
    document: &Value,
) -> Result<String, AdfError> {
    render::to_markdown(document, NoMarkdownForm::Refusal)
}
