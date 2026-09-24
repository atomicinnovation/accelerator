//! The `topic-research` command layer: a set's round plan rendered as the
//! JSON `research-topic`'s `conduct` consumes.

use std::path::Path;

use corpus::scan::DirReader;
use corpus::scan::FileReader;
use corpus::topic_research::round::Round;
use corpus_adapters::topic_research::read_round_inputs;
use serde_json::json;

use crate::outcome::Outcome;

/// Runs `topic-research outstanding` against the set rooted at `set_root`,
/// which must be canonical: each pair's `path` is emitted beneath it.
///
/// # Errors
///
/// A [`kernel::Error`] when the set or the profiles directory cannot be read.
pub fn run_outstanding<F: DirReader + FileReader>(
    set_root: &Path,
    profiles_dir: &Path,
    fs: &F,
) -> Result<Outcome, kernel::Error> {
    let round = Round::plan(&read_round_inputs(set_root, profiles_dir, fs)?);
    Ok(Outcome {
        stdout: format!("{}\n", render(&round, set_root)),
        stderr: String::new(),
    })
}

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
