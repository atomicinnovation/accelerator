//! Adapter/binary wiring for `work resolve`: config resolution, the
//! `Path`/`Invalid` classes (handled directly, no domain call), and the
//! `FullId`/`BareNumber` classes (routed through
//! `work::resolve::resolve`).

use std::path::Path;
use std::path::PathBuf;

use config::ConfigAccess;
use config::Key;
use config_adapters::FileConfigStore;
use corpus::WorkItemIdScheme;
use work::resolve::classify_input;
use work::resolve::resolve as domain_resolve;
use work::resolve::InputClass;
use work::resolve::ResolveOutcome;
use work::resolve::SearchClass;
use work::resolve::TaggedCandidate;
use work_adapters::filesystem::FilesystemLister;

/// The outcome `main` maps to the bash-documented exit codes: `Resolved` →
/// 0, `Ambiguous` → 2, `NotFound`/`Invalid` → 3/1 respectively.
pub enum RunOutcome {
    Resolved(PathBuf),
    Ambiguous(Vec<TaggedCandidate>),
    NotFound(String),
    Invalid(String),
}

fn effective_nonempty(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<String, kernel::Error> {
    let parsed = Key::parse(key)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    Ok(config
        .effective_nonempty(&parsed, None)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?
        .rendered())
}

fn resolve_scheme(
    config: &dyn ConfigAccess,
) -> Result<WorkItemIdScheme, kernel::Error> {
    let id_pattern = effective_nonempty(config, "work.id_pattern")?;
    let default_project_code =
        effective_nonempty(config, "work.default_project_code")?;
    Ok(WorkItemIdScheme {
        id_pattern,
        default_project_code: (!default_project_code.is_empty())
            .then_some(default_project_code),
    })
}

fn resolve_work_dir(
    config: &dyn ConfigAccess,
    root: &Path,
) -> Result<PathBuf, kernel::Error> {
    let dirs = config::paths::doc_type_dirs(config)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    let relative = dirs
        .into_iter()
        .find(|resolved| resolved.doc_type == "work-item")
        .map(|resolved| resolved.dir)
        .ok_or_else(|| {
            kernel::Error::Failed(
                "no work-item doc-type directory configured".to_owned(),
            )
        })?;
    Ok(root.join(relative))
}

fn resolve_path_class(start: &Path, input: &str) -> RunOutcome {
    let candidate = start.join(input);
    match candidate.canonicalize() {
        Ok(resolved) if resolved.is_file() => RunOutcome::Resolved(resolved),
        _ => RunOutcome::NotFound(format!("no work item at path '{input}'")),
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

    match classify_input(input, &scheme) {
        InputClass::Path => Ok(resolve_path_class(start, input)),
        InputClass::Invalid => Ok(RunOutcome::Invalid(format!(
            "input '{input}' is not a recognised path, full ID, or bare \
             number"
        ))),
        class @ (InputClass::FullId | InputClass::BareNumber) => {
            let root = FileConfigStore::discover_root(start);
            let work_dir = resolve_work_dir(config, &root)?;
            let search_class = if class == InputClass::FullId {
                SearchClass::FullId
            } else {
                SearchClass::BareNumber
            };
            let lister = FilesystemLister::new(&work_dir);
            Ok(
                match domain_resolve(input, search_class, &scheme, &lister) {
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
                },
            )
        }
    }
}
