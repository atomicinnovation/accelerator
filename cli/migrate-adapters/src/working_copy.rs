//! Observes the working copy through `vcs_adapters::library::InProcessProbe`,
//! scoped to the migrate engine's own root prefixes.

use std::path::PathBuf;

use migrate::ports::MigrationError;
use migrate::ports::WorkingCopy;
use migrate::ports::WorkingCopyObservation;
use migrate::run_base::RunBase;
use tracing::warn;
use vcs::VcsKind;
use vcs_adapters::library::InProcessProbe;
use vcs_adapters::library::WorkingCopyState;

pub struct VcsWorkingCopy {
    root: PathBuf,
    kind: VcsKind,
}

impl VcsWorkingCopy {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, kind: VcsKind) -> Self {
        Self {
            root: root.into(),
            kind,
        }
    }
}

impl WorkingCopy for VcsWorkingCopy {
    /// A status/diff read that fails (an unreadable or half-initialised
    /// `.git`/`.jj`) is logged and treated as no run base and no changes,
    /// rather than surfaced as a hard error.
    fn observe(
        &self,
        roots: &[&str],
    ) -> Result<WorkingCopyObservation, MigrationError> {
        let state = InProcessProbe
            .working_copy_state(&self.root, self.kind)
            .unwrap_or_else(|error| {
                warn!(
                    root = %self.root.display(),
                    %error,
                    "could not compute the working-copy diff; treating as clean"
                );
                WorkingCopyState::default()
            });
        Ok(WorkingCopyObservation {
            run_base: RunBase::from_base_commits(&state.base_commits),
            dirty_paths: state
                .dirty_paths
                .into_iter()
                .filter(|path| {
                    roots.iter().any(|scope| path.starts_with(scope))
                })
                .collect(),
        })
    }
}
