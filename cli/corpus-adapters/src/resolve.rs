//! Resolving a slug or a path to a document's root on disk.
//!
//! The `Path` class (canonicalise, containment, nested set-root walk) is
//! handled directly, and the `Slug` class routed through
//! [`corpus::resolve::resolve`].

use std::path::Path;
use std::path::PathBuf;

use corpus::resolve::classify_input;
use corpus::resolve::resolve as domain_resolve;
use corpus::resolve::InputClass;
use corpus::resolve::ResolveOutcome;
use corpus::resolve::TaggedCandidate;
use corpus::resolve::TypeShape;
use corpus::DocTypeKey;

use crate::fs::TypeDirectoryLister;

pub enum Resolution {
    Resolved(PathBuf),
    Ambiguous(Vec<TaggedCandidate>),
    NotFound(String),
    Invalid(String),
    UnknownType(String),
    OutsideRoot(String),
}

/// Resolves `slug` for `doc_type`, relative to `start`, to the document root
/// — a file for a flat type, the set directory for a nested-manifest type.
///
/// `type_dir` is asked for the type's configured directory only once `slug`
/// is known to be a slug or a path, so an invalid input never depends on the
/// configuration or the filesystem.
pub fn resolve_document(
    start: &Path,
    doc_type: &str,
    slug: &str,
    type_dir: impl FnOnce(DocTypeKey) -> Result<PathBuf, String>,
) -> Resolution {
    let Some(key) = DocTypeKey::from_linkage_type_name(doc_type) else {
        return Resolution::UnknownType(format!(
            "'{doc_type}' is not a registered document type"
        ));
    };
    let shape = if key.nested_manifest_filename().is_some() {
        TypeShape::NestedManifest
    } else {
        TypeShape::Flat
    };

    let class = classify_input(slug);
    if class == InputClass::Invalid {
        return Resolution::Invalid(format!(
            "input '{slug}' is not a recognised slug or path"
        ));
    }
    let root = match canonical_type_root(type_dir(key), doc_type) {
        Ok(root) => root,
        Err(outcome) => return outcome,
    };
    if class == InputClass::Path {
        return resolve_path_class(&root, start, slug, shape, doc_type);
    }
    let lister = TypeDirectoryLister::new(root.clone(), shape);
    match domain_resolve(slug, shape, &lister) {
        ResolveOutcome::Single(name) => Resolution::Resolved(root.join(name)),
        ResolveOutcome::Ambiguous(candidates) => {
            Resolution::Ambiguous(candidates)
        }
        ResolveOutcome::NotFound => Resolution::NotFound(format!(
            "no '{doc_type}' matching slug '{slug}' in {}",
            root.display()
        )),
    }
}

/// The canonicalised type directory, or the [`Resolution`] to emit when it
/// cannot be resolved (a config error) or does not exist on disk.
fn canonical_type_root(
    type_dir: Result<PathBuf, String>,
    doc_type: &str,
) -> Result<PathBuf, Resolution> {
    let type_dir = type_dir.map_err(Resolution::Invalid)?;
    type_dir.canonicalize().map_err(|_| {
        Resolution::NotFound(format!(
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
) -> Resolution {
    let candidate = start.join(input);
    let Ok(resolved) = candidate.canonicalize() else {
        return Resolution::NotFound(format!(
            "no '{doc_type}' at path '{input}'"
        ));
    };
    if !resolved.starts_with(root) {
        return Resolution::OutsideRoot(format!(
            "path '{input}' is outside the '{doc_type}' directory {}",
            root.display()
        ));
    }
    match shape {
        TypeShape::Flat => {
            if resolved.is_file() {
                Resolution::Resolved(resolved)
            } else {
                Resolution::NotFound(format!(
                    "no '{doc_type}' at path '{input}'"
                ))
            }
        }
        TypeShape::NestedManifest => set_root_under(root, &resolved)
            .map_or_else(
                || {
                    Resolution::NotFound(format!(
                        "no '{doc_type}' set at path '{input}'"
                    ))
                },
                Resolution::Resolved,
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
