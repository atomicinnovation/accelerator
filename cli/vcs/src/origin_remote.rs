//! The `origin` remote's configured URL.

use std::path::Path;

/// Reads a repository's configured `origin` remote URL.
///
/// Deliberately just the raw URL string: which hosted forge (if any) it
/// names, and how to parse an owner/repo out of it, are not a `vcs`
/// concern — `vcs` is idiom-agnostic (git vs jj), not hosting-agnostic. That
/// parsing lives in `collaboration`, behind its own port.
pub trait OriginRemote {
    /// The `origin` remote's URL, `Ok(None)` when no `origin` remote is
    /// configured, or `Err` when the probe itself could not answer —
    /// callers must be able to distinguish "cleanly absent" from "probe
    /// malfunctioned", unlike `RepoRoot`/`VcsProbe`'s infallible
    /// fold-to-`None` convention.
    ///
    /// # Errors
    ///
    /// When the repository cannot be opened or its remote configuration
    /// cannot be read.
    fn origin_url(&self, root: &Path) -> Result<Option<String>, kernel::Error>;
}
