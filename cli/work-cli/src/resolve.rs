//! Adapter/binary wiring for `work resolve`: config resolution, the
//! `Path`/`Invalid` classes (handled directly, no domain call), and the
//! `FullId`/`BareNumber` classes (routed through
//! `work::resolve::resolve`).

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use config_adapters::FileConfigStore;
use corpus::WorkItemIdScheme;
use work::resolve::classify_input;
use work::resolve::resolve as domain_resolve;
use work::resolve::InputClass;
use work::resolve::ResolveOutcome;
use work::resolve::SearchClass;
use work::resolve::TaggedCandidate;
use work_adapters::filesystem::FilesystemLister;

use crate::config::resolve_scheme;
use crate::config::resolve_work_dir;

/// The outcome `main` maps to the binary's exit codes: `Resolved` → 0,
/// `Ambiguous` → 2, `NotFound` → 3, `Invalid` → 1, `OutsideWorkDir` → 6.
pub enum RunOutcome {
    Resolved(PathBuf),
    Ambiguous(Vec<TaggedCandidate>),
    NotFound(String),
    Invalid(String),
    OutsideWorkDir(String),
}

fn resolve_path_class(root: &Path, start: &Path, input: &str) -> RunOutcome {
    let candidate = start.join(input);
    let Ok(resolved) = candidate.canonicalize() else {
        return RunOutcome::NotFound(format!("no work item at path '{input}'"));
    };
    if !resolved.starts_with(root) {
        return RunOutcome::OutsideWorkDir(format!(
            "path '{input}' is outside the work directory {}",
            root.display()
        ));
    }
    if resolved.is_file() {
        RunOutcome::Resolved(resolved)
    } else {
        RunOutcome::NotFound(format!("no work item at path '{input}'"))
    }
}

/// # Errors
///
/// A [`kernel::Error`] when the configuration cannot be read.
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    input: &str,
) -> Result<RunOutcome, kernel::Error> {
    let scheme = resolve_scheme(config)?;
    let work_dir = canonical_work_dir(start, config)?;
    Ok(resolve_with(&scheme, &work_dir, start, input))
}

/// Resolves the canonical work directory once, so a caller resolving many
/// tokens reads config a single time and then resolves each infallibly through
/// [`resolve_with`].
///
/// # Errors
///
/// A [`kernel::Error`] when the configuration cannot be read or the work
/// directory cannot be canonicalised — an environment fault, not a missing
/// item.
pub fn canonical_work_dir(
    start: &Path,
    config: &dyn ConfigAccess,
) -> Result<PathBuf, kernel::Error> {
    let root = FileConfigStore::discover_root(start);
    let work_dir = resolve_work_dir(config, &root)?;
    work_dir.canonicalize().map_err(|error| {
        kernel::Error::Failed(format!(
            "could not resolve the work directory {}: {error}",
            work_dir.display()
        ))
    })
}

/// The infallible core: classify `input` and resolve it against the already
/// resolved `scheme` and canonical `work_dir`. `Resolved` carries a
/// canonicalised path so a caller can match it against an equally-canonicalised
/// managed-item set.
#[must_use]
pub fn resolve_with(
    scheme: &WorkItemIdScheme,
    work_dir: &Path,
    start: &Path,
    input: &str,
) -> RunOutcome {
    match classify_input(input, scheme) {
        InputClass::Path => resolve_path_class(work_dir, start, input),
        InputClass::Invalid => RunOutcome::Invalid(format!(
            "input '{input}' is not a recognised path, full ID, or bare \
             number"
        )),
        class @ (InputClass::FullId | InputClass::BareNumber) => {
            let search_class = if class == InputClass::FullId {
                SearchClass::FullId
            } else {
                SearchClass::BareNumber
            };
            let lister = FilesystemLister::new(work_dir);
            match domain_resolve(input, search_class, scheme, &lister) {
                ResolveOutcome::Single(filename) => {
                    RunOutcome::Resolved(work_dir.join(filename))
                }
                ResolveOutcome::Ambiguous(candidates) => {
                    RunOutcome::Ambiguous(candidates)
                }
                ResolveOutcome::NotFound => {
                    let noun = if search_class == SearchClass::FullId {
                        "ID"
                    } else {
                        "bare number"
                    };
                    RunOutcome::NotFound(format!(
                        "no work item matching {noun} '{input}' in {}",
                        work_dir.display()
                    ))
                }
            }
        }
    }
}
