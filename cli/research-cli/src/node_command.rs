//! The read-only `node` command: the context a `conduct` run offered one
//! node's researcher, as its ledger's snapshot holds it.

use std::path::Path;

use corpus::scan::FileReader;
use research::conduct::ledger::RunId;
use research::conduct::spawn::SpawnRef;
use research::topic::layout::note_ref::NoteRef;
use serde_json::json;

use crate::ledger_wording;
use crate::node_address;
use crate::topic_command::owned_ledger;
use crate::topic_command::Printed;

/// # Errors
///
/// A message when `text` is not a node's spawn ref.
pub fn parse_node(text: &str) -> Result<NoteRef, String> {
    match SpawnRef::parse(text) {
        Some(SpawnRef::Node(at)) => Ok(at),
        Some(SpawnRef::Pair(_)) | None => Err(format!(
            "E_TOPIC_RESEARCH_NODE: --node must be a node's spawn ref, \
             <stem>:<lineage>, got '{text}'"
        )),
    }
}

/// Prints the context the node at `at` was offered with in `run`.
///
/// # Errors
///
/// A message when `run` has no usable ledger, another run owns it, or its
/// current offered batch holds no context for the node.
pub fn run_node<F: FileReader>(
    set_root: &Path,
    run: &RunId,
    at: &NoteRef,
    fs: &F,
) -> Result<Printed, String> {
    let ledger = owned_ledger(set_root, run, fs)?
        .ok_or_else(|| ledger_wording::no_usable_ledger(run))?;
    let context = ledger.offered_context(at).ok_or_else(|| {
        format!(
            "E_TOPIC_RESEARCH_NODE_NOT_OFFERED: node {at} is not in run \
             {run}'s current offered batch, which conduct may have \
             acknowledged before this query ran; re-run conduct"
        )
    })?;
    let mut rendered = node_address::addressed(set_root, at);
    rendered["question"] = json!(context.question);
    rendered["known_questions"] = json!(context.known_questions);
    let level = at.lineage.level();
    if let Some(budget) = ledger.search_budget(&context.profile, level) {
        rendered["search_budget"] = json!(budget);
    }
    Ok(Printed {
        stdout: format!("{rendered}\n"),
        stderr: String::new(),
    })
}
