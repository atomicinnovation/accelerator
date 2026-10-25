use std::path::PathBuf;

use migrate::ports::MigrationError;
use migrate::ports::WorkingCopy;
use migrate::ports::WorkingCopyObservation;
use migrate::run_base::RunBase;
use tracing::warn;
use vcs::guarded_working_copy::observe_guarded;
use vcs::guarded_working_copy::working_copy_seen_from;
use vcs::guarded_working_copy::GuardedObservation;
use vcs::RepositoryProbe;
use vcs::WorkingCopyState;

pub struct VcsWorkingCopy<'a> {
    root: PathBuf,
    probe: &'a dyn RepositoryProbe,
}

impl<'a> VcsWorkingCopy<'a> {
    #[must_use]
    pub fn new(
        root: impl Into<PathBuf>,
        probe: &'a dyn RepositoryProbe,
    ) -> Self {
        Self {
            root: root.into(),
            probe,
        }
    }

    fn probed(&self) -> Result<WorkingCopyState, kernel::Error> {
        working_copy_seen_from(self.probe, &self.root)
            .map(Option::unwrap_or_default)
    }
}

impl WorkingCopy for VcsWorkingCopy<'_> {
    /// A status/diff read that fails (an unreadable or half-initialised
    /// `.git`/`.jj`) is logged and treated as no run base and no changes,
    /// rather than surfaced as a hard error.
    fn observe(
        &self,
        roots: &[&str],
    ) -> Result<WorkingCopyObservation, MigrationError> {
        let state = match observe_guarded(self.probed(), roots) {
            GuardedObservation::Observed(state) => state,
            GuardedObservation::Unprobeable { reason } => {
                warn!(
                    root = %self.root.display(),
                    error = %reason,
                    "could not compute the working-copy diff; treating as clean"
                );
                WorkingCopyState::default()
            }
        };
        Ok(WorkingCopyObservation {
            run_base: RunBase::from_base_commits(&state.base_commits),
            dirty_paths: state.dirty_paths,
        })
    }
}
