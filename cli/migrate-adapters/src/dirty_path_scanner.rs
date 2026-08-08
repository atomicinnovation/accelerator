//! Scopes `vcs_adapters::library::InProcessProbe::dirty_paths` to the
//! migrate engine's own root prefixes.

use std::path::PathBuf;

use migrate::ports::DirtyPathScanner;
use migrate::ports::MigrationError;
use tracing::warn;
use vcs::VcsKind;
use vcs_adapters::library::InProcessProbe;

pub struct VcsDirtyPathScanner {
    root: PathBuf,
    kind: VcsKind,
}

impl VcsDirtyPathScanner {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, kind: VcsKind) -> Self {
        Self {
            root: root.into(),
            kind,
        }
    }
}

impl DirtyPathScanner for VcsDirtyPathScanner {
    /// Mirrors `enumerate_scoped_dirty`'s trailing `|| true`: a status/diff
    /// read that fails (an unreadable or half-initialised `.git`/`.jj`) is
    /// logged and treated as no dirty paths found, not a hard error —
    /// exactly as the bash pipeline's suppressed exit status did.
    fn dirty_paths(
        &self,
        roots: &[&str],
    ) -> Result<Vec<String>, MigrationError> {
        let probe = InProcessProbe;
        let all = probe.dirty_paths(&self.root, self.kind).unwrap_or_else(
            |error| {
                warn!(
                    root = %self.root.display(),
                    %error,
                    "could not compute the working-copy diff; treating as clean"
                );
                Vec::new()
            },
        );
        Ok(all
            .into_iter()
            .filter(|path| roots.iter().any(|scope| path.starts_with(scope)))
            .collect())
    }
}
