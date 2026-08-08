//! The clock and VCS-facts ports, and the artifact-metadata output type.

use std::path::Path;

/// A filename-timestamp format, one per subsumed metadata helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilenameTimestampFormat {
    DateTimeUnderscored,
    DateOnly,
    CompactTime,
}

/// The wall-clock seam: UTC ISO for the datetime line, host-local for the
/// filename timestamp. Faked in tests so every derived field is deterministic.
pub trait Clock {
    fn now_utc_iso(&self) -> String;
    fn filename_timestamp(&self, format: FilenameTimestampFormat) -> String;
}

/// A repository's name and current revision, in this crate's own vocabulary.
///
/// [`RepoFactsProbe`]'s concrete implementation translates from whatever
/// VCS-specific type it reads, so nothing inward of that translation needs
/// to depend on a VCS crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryFacts {
    pub name: String,
    pub revision: Option<String>,
}

/// The VCS-facts seam: the repository facts for the checkout containing
/// `start`, or `None` outside one.
///
/// Injected so `corpus-adapters`' metadata-composing functions never call a
/// sibling adapter crate directly, and a test can supply fixed facts
/// without a real repository.
pub trait RepoFactsProbe {
    fn facts(&self, start: &Path) -> Option<RepositoryFacts>;
}

/// The facts an artifact-metadata block carries. `repository_name`/`revision`
/// are absent outside a VCS checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactMetadata {
    pub datetime_utc: String,
    pub filename_timestamp: String,
    pub repository_name: Option<String>,
    pub revision: Option<String>,
}
