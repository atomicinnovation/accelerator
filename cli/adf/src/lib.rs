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
mod content_model;
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
    Unrepresentable(Unrepresentable),
    DisallowedNesting { child: String, container: String },
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
            Self::Unrepresentable(reason) => write!(
                formatter,
                "E_ADF_UNREPRESENTABLE: {reason} has no Markdown form"
            ),
            Self::DisallowedNesting { child, container } => write!(
                formatter,
                "E_ADF_UNSUPPORTED: Jira does not allow a {child} inside a \
                 {container}"
            ),
            Self::BadInput => write!(
                formatter,
                "E_ADF_BAD_INPUT: input contains control byte \\x1e or \\x1f"
            ),
        }
    }
}

impl Error for AdfError {}

/// Content a strict render refuses because no Markdown it could emit reads
/// back as the same document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unrepresentable {
    Node { node: String },
    HeadingBeyondLevelSix,
    FenceLineInCodeBlock,
    BacktickInInlineCode,
    PipeInTableCellCode,
    PipeInTableCellLink,
    HardBreakInTableCell,
    TableCellOfSeveralBlocks,
    LinkToCodeRunningAddress,
}

impl fmt::Display for Unrepresentable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node { node } => write!(formatter, "a {node}"),
            Self::HeadingBeyondLevelSix => {
                formatter.write_str("a heading beyond level six")
            }
            Self::FenceLineInCodeBlock => formatter.write_str(
                "a code block holding a line that starts with three backticks",
            ),
            Self::BacktickInInlineCode => {
                formatter.write_str("inline code holding a backtick")
            }
            Self::PipeInTableCellCode => formatter
                .write_str("inline code holding a pipe inside a table cell"),
            Self::PipeInTableCellLink => formatter.write_str(
                "a link whose address holds a pipe inside a table cell",
            ),
            Self::HardBreakInTableCell => {
                formatter.write_str("a line break inside a table cell")
            }
            Self::TableCellOfSeveralBlocks => formatter
                .write_str("a table cell holding more than one paragraph"),
            Self::LinkToCodeRunningAddress => formatter.write_str(
                "a link to a javascript:, vbscript: or data: address",
            ),
        }
    }
}

/// The whole markdown-to-ADF pipeline: tokenise, then assemble.
///
/// `seed` selects the localid form: `Some` gives the deterministic
/// `00000000-0000-4000-8000-00000000000N` form, `None` the bare counter. It is
/// a parameter rather than an environment read so a test needs no process
/// state.
///
/// # Errors
///
/// [`AdfError::BadInput`] for a control byte;
/// [`AdfError::DisallowedNesting`] for a block Jira does not allow inside
/// its blockquote or list item.
pub fn markdown_to_document(
    markdown: &str,
    seed: Option<&str>,
) -> Result<Value, AdfError> {
    let tokenised = tokenise::tokenise(markdown)?;
    let document = assemble::to_document(&tokenised.tokens, seed);
    content_model::admit(&document)?;
    Ok(document)
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
/// [`AdfError::Unrepresentable`] at the first content with no Markdown form,
/// naming it as an [`Unrepresentable`], and every error
/// [`document_to_markdown`] raises.
pub fn document_to_markdown_strict(
    document: &Value,
) -> Result<String, AdfError> {
    render::to_markdown(document, NoMarkdownForm::Refusal)
}
