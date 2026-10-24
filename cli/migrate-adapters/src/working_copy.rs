use std::path::Path;
use std::path::PathBuf;

use migrate::ports::MigrationError;
use migrate::ports::WorkingCopy;
use migrate::ports::WorkingCopyObservation;
use migrate::run_base::RunBase;
use tracing::warn;
use vcs::guarded_working_copy::observe_guarded;
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

    /// The working copy containing `root`, its dirty paths relative to
    /// `root`, which may sit below the repository's own root.
    fn probed(&self) -> Result<WorkingCopyState, kernel::Error> {
        let Some(facts) = self.probe.facts_at(&self.root) else {
            return Ok(WorkingCopyState::default());
        };
        let canonical = |path: &Path| {
            path.canonicalize().unwrap_or_else(|_| path.to_owned())
        };
        let config_root = canonical(&self.root);
        let within =
            config_root
                .strip_prefix(canonical(&facts.root))
                .map_err(|_| {
                    kernel::Error::Failed(format!(
                        "{} lies outside the repository at {}",
                        self.root.display(),
                        facts.root.display()
                    ))
                })?;
        self.probe
            .working_copy_state(&facts.root, facts.kind)
            .map(|state| state.seen_from(within))
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
