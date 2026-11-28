//! What every `research topic` subcommand shares: resolving the set a slug
//! names, and the output one call prints.

use std::path::Path;
use std::path::PathBuf;

use corpus::DocTypeKey;
use corpus_adapters::resolve::resolve_document;
use corpus_adapters::resolve::Resolution;

use crate::context::ProjectContext;

const DOC_TYPE: &str = "topic-research";

/// What one call prints: the plan on stdout, its warnings on stderr.
#[derive(Debug, Default)]
pub struct Printed {
    pub stdout: String,
    pub stderr: String,
}

/// Why no set root was resolved.
#[derive(Debug)]
pub enum SetError {
    UnreadableConfig(String),
    Unresolved(String),
}

impl From<SetError> for String {
    fn from(error: SetError) -> Self {
        match error {
            SetError::UnreadableConfig(message)
            | SetError::Unresolved(message) => message,
        }
    }
}

/// The canonical root of the set `slug` names.
///
/// # Errors
///
/// [`SetError::UnreadableConfig`] when the configured directory cannot be
/// read or the doc type catalogue cannot be assembled, and
/// [`SetError::Unresolved`] when `slug` resolves to no set.
pub fn resolve_set(
    project: &ProjectContext,
    cwd: &Path,
    slug: &str,
) -> Result<PathBuf, SetError> {
    let sets = project
        .type_dir(DocTypeKey::TOPIC_RESEARCH)
        .map_err(SetError::UnreadableConfig)?;
    let catalogue = declarations::catalogue()
        .map_err(|error| SetError::UnreadableConfig(error.to_string()))?;
    let Resolution::Resolved(set_root) =
        resolve_document(&catalogue, cwd, DOC_TYPE, slug, |_| Ok(sets))
    else {
        return Err(SetError::Unresolved(format!(
            "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set '{slug}'"
        )));
    };
    Ok(set_root)
}
