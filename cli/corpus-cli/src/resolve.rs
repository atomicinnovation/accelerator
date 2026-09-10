//! Adapter wiring for `corpus resolve`: type-directory resolution, the `Path`
//! class (canonicalise, containment, nested set-root walk) handled directly,
//! and the `Slug` class routed through [`corpus::resolve::resolve`].

use std::path::Path;
use std::path::PathBuf;

use corpus::resolve::classify_input;
use corpus::resolve::resolve as domain_resolve;
use corpus::resolve::DirectoryLister;
use corpus::resolve::InputClass;
use corpus::resolve::ResolveOutcome;
use corpus::resolve::TaggedCandidate;
use corpus::resolve::TypeShape;
use corpus::DocTypeKey;

use crate::config::resolve_type_dir;
use crate::config::Composed;

/// The outcome `main` maps to the binary's exit codes: `Resolved` → 0,
/// `Invalid` → 1, `Ambiguous` → 2, `NotFound` → 3, `UnknownType` → 4,
/// `OutsideRoot` → 6.
pub enum RunOutcome {
    Resolved(PathBuf),
    Ambiguous(Vec<TaggedCandidate>),
    NotFound(String),
    Invalid(String),
    UnknownType(String),
    OutsideRoot(String),
}

/// Lists the type directory's entries for the domain: `.md` files for a flat
/// type, immediate subdirectories for a nested-manifest type.
struct TypeDirectoryLister {
    dir: PathBuf,
    shape: TypeShape,
}

impl DirectoryLister for TypeDirectoryLister {
    fn entries(&self) -> Vec<String> {
        let Ok(reader) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        reader
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let file_type = entry.file_type().ok()?;
                let name = entry.file_name().to_str()?.to_owned();
                let keep = match self.shape {
                    TypeShape::Flat => {
                        file_type.is_file() && is_markdown(&name)
                    }
                    TypeShape::NestedManifest => file_type.is_dir(),
                };
                keep.then_some(name)
            })
            .collect()
    }
}

fn is_markdown(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// Resolves `slug` for `doc_type` against `start`'s project, returning the
/// document root — a file for a flat type, the set directory for a
/// nested-manifest type.
pub fn run(
    start: &Path,
    composed: &Composed,
    doc_type: &str,
    slug: &str,
) -> RunOutcome {
    let Some(key) = DocTypeKey::from_linkage_type_name(doc_type) else {
        return RunOutcome::UnknownType(format!(
            "'{doc_type}' is not a registered document type"
        ));
    };
    let shape = if key.nested_manifest_filename().is_some() {
        TypeShape::NestedManifest
    } else {
        TypeShape::Flat
    };

    // Classify first: an empty input is invalid whether or not the type
    // directory exists, so it must never depend on the filesystem probe below.
    match classify_input(slug) {
        InputClass::Invalid => RunOutcome::Invalid(format!(
            "input '{slug}' is not a recognised slug or path"
        )),
        InputClass::Path => {
            let root = match canonical_type_root(composed, key, doc_type) {
                Ok(root) => root,
                Err(outcome) => return outcome,
            };
            resolve_path_class(&root, start, slug, shape, doc_type)
        }
        InputClass::Slug => {
            let root = match canonical_type_root(composed, key, doc_type) {
                Ok(root) => root,
                Err(outcome) => return outcome,
            };
            let lister = TypeDirectoryLister {
                dir: root.clone(),
                shape,
            };
            match domain_resolve(slug, shape, &lister) {
                ResolveOutcome::Single(name) => {
                    RunOutcome::Resolved(root.join(name))
                }
                ResolveOutcome::Ambiguous(candidates) => {
                    RunOutcome::Ambiguous(candidates)
                }
                ResolveOutcome::NotFound => RunOutcome::NotFound(format!(
                    "no '{doc_type}' matching slug '{slug}' in {}",
                    root.display()
                )),
            }
        }
    }
}

/// The canonicalised type directory, or the [`RunOutcome`] to emit when it
/// cannot be resolved (a config error) or does not exist on disk.
fn canonical_type_root(
    composed: &Composed,
    key: DocTypeKey,
    doc_type: &str,
) -> Result<PathBuf, RunOutcome> {
    let type_dir = resolve_type_dir(composed, key)
        .map_err(|error| RunOutcome::Invalid(error.to_string()))?;
    type_dir.canonicalize().map_err(|_| {
        RunOutcome::NotFound(format!(
            "no '{doc_type}' directory at {}",
            type_dir.display()
        ))
    })
}

fn resolve_path_class(
    root: &Path,
    start: &Path,
    input: &str,
    shape: TypeShape,
    doc_type: &str,
) -> RunOutcome {
    let candidate = start.join(input);
    let Ok(resolved) = candidate.canonicalize() else {
        return RunOutcome::NotFound(format!(
            "no '{doc_type}' at path '{input}'"
        ));
    };
    if !resolved.starts_with(root) {
        return RunOutcome::OutsideRoot(format!(
            "path '{input}' is outside the '{doc_type}' directory {}",
            root.display()
        ));
    }
    match shape {
        TypeShape::Flat => {
            if resolved.is_file() {
                RunOutcome::Resolved(resolved)
            } else {
                RunOutcome::NotFound(format!(
                    "no '{doc_type}' at path '{input}'"
                ))
            }
        }
        TypeShape::NestedManifest => set_root_under(root, &resolved)
            .map_or_else(
                || {
                    RunOutcome::NotFound(format!(
                        "no '{doc_type}' set at path '{input}'"
                    ))
                },
                RunOutcome::Resolved,
            ),
    }
}

/// The set root for a target somewhere under `root`: the first path segment
/// directly under the type directory. A sub-document lives in a subdirectory
/// (`findings/`, `screenshots/`) whose immediate parent is not the set root, so
/// this walks up until the parent is exactly `root` rather than taking the
/// immediate parent.
fn set_root_under(root: &Path, target: &Path) -> Option<PathBuf> {
    let mut current = target;
    loop {
        let parent = current.parent()?;
        if parent == root {
            return Some(current.to_path_buf());
        }
        current = parent;
    }
}
