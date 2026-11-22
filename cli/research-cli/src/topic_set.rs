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

/// The canonical root of the set `slug` names.
///
/// # Errors
///
/// A message when the configured directory cannot be read, or when `slug`
/// resolves to no set.
pub fn resolve_set(
    project: &ProjectContext,
    cwd: &Path,
    slug: &str,
) -> Result<PathBuf, String> {
    let sets = project.type_dir(DocTypeKey::TopicResearch)?;
    let Resolution::Resolved(set_root) =
        resolve_document(cwd, DOC_TYPE, slug, |_| Ok(sets))
    else {
        return Err(format!(
            "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set '{slug}'"
        ));
    };
    Ok(set_root)
}
