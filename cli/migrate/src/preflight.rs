//! Clean-tree pre-flight: acquire the run lock, scan for VCS-dirty scoped
//! paths, and classify them against the path manifest. `--list` skips this
//! module entirely — it is never called on that path.

use crate::manifest::classify;
use crate::manifest::is_runner_managed;
use crate::manifest::is_session_log;
use crate::manifest::Ownership;
use crate::manifest::RunnerPaths;
use crate::ports::ManifestStore;
use crate::ports::MigrationError;
use crate::ports::RunLock;
use crate::ports::RunLockGuard;
use crate::ports::WorkingCopy;
use crate::run_base::RunBase;

pub const SCOPES: [&str; 3] = ["meta/", ".claude/accelerator", ".accelerator/"];

pub struct AffordanceEntry {
    pub path: String,
    pub session_log_decision_count: Option<usize>,
}

pub enum PreflightOutcome {
    Clean,
    Resumed { affordance: Vec<AffordanceEntry> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnownedChanges {
    pub paths: Vec<String>,
    pub stale_run: bool,
}

#[derive(Debug)]
pub enum PreflightError {
    UnownedChanges(UnownedChanges),
    Failed(MigrationError),
}

impl std::fmt::Display for PreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnownedChanges(_) => write!(f, "dirty working tree"),
            Self::Failed(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PreflightError {}

impl From<MigrationError> for PreflightError {
    fn from(error: MigrationError) -> Self {
        Self::Failed(error)
    }
}

pub struct Preflight<'a> {
    pub lock: &'a dyn RunLock,
    pub working_copy: &'a dyn WorkingCopy,
    pub manifest: &'a dyn ManifestStore,
    pub runner: RunnerPaths<'a>,
    pub force: bool,
    pub session_log_decision_count: &'a dyn Fn(&str) -> usize,
}

impl Preflight<'_> {
    /// # Errors
    /// [`PreflightError::UnownedChanges`] when the tree is dirty outside this
    /// run's own manifest/session artefacts; [`PreflightError::Failed`] when
    /// the lock, scan, or manifest I/O itself fails.
    pub fn run(
        &self,
    ) -> Result<(RunLockGuard, PreflightOutcome), PreflightError> {
        let guard = self.lock.acquire()?;
        let observation = self.working_copy.observe(&SCOPES)?;
        let run_base = observation.run_base.as_ref();

        if self.force {
            self.start_fresh_run(run_base)?;
            return Ok((guard, PreflightOutcome::Clean));
        }

        let dirty: Vec<String> = observation
            .dirty_paths
            .into_iter()
            .filter(|path| !is_runner_managed(path, &self.runner))
            .collect();
        if dirty.is_empty() {
            self.start_fresh_run(run_base)?;
            return Ok((guard, PreflightOutcome::Clean));
        }

        let unowned = self.unowned_changes(&dirty, run_base)?;
        if !unowned.paths.is_empty() {
            return Err(PreflightError::UnownedChanges(unowned));
        }

        let affordance = dirty
            .iter()
            .map(|path| AffordanceEntry {
                path: path.clone(),
                session_log_decision_count: is_session_log(path)
                    .then(|| (self.session_log_decision_count)(path)),
            })
            .collect();
        Ok((guard, PreflightOutcome::Resumed { affordance }))
    }

    fn start_fresh_run(
        &self,
        run_base: Option<&RunBase>,
    ) -> Result<(), PreflightError> {
        self.manifest.write_manifest(&[])?;
        self.manifest.record_run_base(run_base)?;
        Ok(())
    }

    fn unowned_changes(
        &self,
        dirty: &[String],
        run_base: Option<&RunBase>,
    ) -> Result<UnownedChanges, PreflightError> {
        let manifest = self.manifest.manifest()?;
        let recorded = self.manifest.recorded_run_base()?;
        let current_run = recorded.is_some() && recorded.as_ref() == run_base;
        let stale_run =
            manifest.is_some() && recorded.is_some() && !current_run;
        let mut paths: Vec<String> = match manifest {
            Some(manifest) if current_run => dirty
                .iter()
                .filter(|path| {
                    classify(path, &self.runner, &manifest, current_run)
                        == Ownership::Unowned
                })
                .cloned()
                .collect(),
            _ => dirty.to_vec(),
        };
        paths.sort_unstable();
        Ok(UnownedChanges { paths, stale_run })
    }
}
