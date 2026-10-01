//! One `research topic outstanding` call: a set's round plan rendered as the
//! JSON `research-topic`'s `conduct` consumes.

use std::path::Path;

use corpus::scan::DirReader;
use corpus::scan::FileReader;
use corpus_adapters::resolve::resolve_document;
use corpus_adapters::resolve::Resolution;
use research::round::Round;
use research_adapters::topic_research::read_round_inputs;
use serde_json::json;

use crate::context::ProjectContext;

const DOC_TYPE: &str = "topic-research";

/// The round plan of the set `slug` names, as one line of JSON.
///
/// # Errors
///
/// A message when `slug` resolves to no set, or the set or the profiles
/// directory cannot be read.
pub fn outstanding<F: DirReader + FileReader>(
    project: &ProjectContext,
    cwd: &Path,
    slug: &str,
    profiles_dir: &Path,
    fs: &F,
) -> Result<String, String> {
    let Resolution::Resolved(set_root) =
        resolve_document(cwd, DOC_TYPE, slug, |key| project.type_dir(key))
    else {
        return Err(format!(
            "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set '{slug}'"
        ));
    };
    let inputs = read_round_inputs(&set_root, profiles_dir, fs)
        .map_err(|error| error.to_string())?;
    Ok(render(&Round::plan(&inputs), &set_root).to_string())
}

/// Each pair's `path` is emitted beneath `set_root`, which resolution has
/// already made canonical.
fn render(round: &Round, set_root: &Path) -> serde_json::Value {
    let items: Vec<_> = round
        .items
        .iter()
        .map(|item| {
            json!({
                "line": item.line,
                "question": item.question,
                "complete": item.complete,
            })
        })
        .collect();
    let pairs: Vec<_> = round
        .pairs
        .iter()
        .map(|pair| {
            json!({
                "question": pair.question,
                "profile": pair.profile,
                "path": set_root.join(&pair.path).to_string_lossy(),
            })
        })
        .collect();
    let skipped: Vec<_> = round
        .skipped
        .iter()
        .map(|skip| {
            json!({
                "question": skip.question,
                "profile": skip.profile,
                "reason": skip.explanation(),
            })
        })
        .collect();
    let warnings: Vec<_> =
        round.warnings.iter().map(ToString::to_string).collect();
    json!({
        "items": items,
        "pairs": pairs,
        "skipped": skipped,
        "warnings": warnings,
    })
}
