//! Adapter wiring for `corpus resolve`: the configured type directory handed
//! to [`corpus_adapters::resolve::resolve_document`].

use std::path::Path;

use corpus_adapters::resolve::resolve_document;
use corpus_adapters::resolve::Resolution;

use crate::config::resolve_type_dir;
use crate::config::Composed;

/// Resolves `slug` for `doc_type` against `start`'s project. `main` maps the
/// [`Resolution`] to the binary's exit codes: `Resolved` → 0, `Invalid` → 1,
/// `Ambiguous` → 2, `NotFound` → 3, `UnknownType` → 4, `OutsideRoot` → 6.
pub fn run(
    start: &Path,
    composed: &Composed,
    doc_type: &str,
    slug: &str,
) -> Resolution {
    resolve_document(start, doc_type, slug, |key| {
        resolve_type_dir(composed, key).map_err(|error| error.to_string())
    })
}
