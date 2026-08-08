//! Clean-tree pre-flight: acquire the run lock, scan for VCS-dirty scoped
//! paths, and classify them against the path manifest. `--list` skips this
//! module entirely — it is never called on that path.

use crate::manifest::classify;
use crate::manifest::is_session_log;
use crate::manifest::Ownership;
use crate::manifest::RunnerPaths;
use crate::ports::DirtyPathScanner;
use crate::ports::ManifestStore;
use crate::ports::MigrationError;
use crate::ports::RunLock;
use crate::ports::RunLockGuard;

pub const SCOPES: [&str; 3] = ["meta/", ".claude/accelerator", ".accelerator/"];

pub struct AffordanceEntry {
    pub path: String,
    pub session_log_decision_count: Option<usize>,
}

pub enum PreflightOutcome {
    Clean,
    Resumed { affordance: Vec<AffordanceEntry> },
}

#[derive(Debug)]
pub enum PreflightError {
    ForeignDirt,
    Failed(MigrationError),
}

impl std::fmt::Display for PreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignDirt => write!(f, "dirty working tree"),
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
    pub scanner: &'a dyn DirtyPathScanner,
    pub manifest: &'a dyn ManifestStore,
    pub runner: RunnerPaths<'a>,
    pub revision: Option<String>,
    pub force: bool,
    pub session_log_decision_count: &'a dyn Fn(&str) -> usize,
}

impl Preflight<'_> {
    /// # Errors
    /// [`PreflightError::ForeignDirt`] when the tree is dirty outside this
    /// run's own manifest/session artefacts; [`PreflightError::Failed`] when
    /// the lock, scan, or manifest I/O itself fails.
    pub fn run(
        &self,
    ) -> Result<(RunLockGuard, PreflightOutcome), PreflightError> {
        let guard = self.lock.acquire()?;

        if self.force {
            self.manifest.write_manifest(&[])?;
            self.manifest.write_run_id(self.revision.as_deref())?;
            return Ok((guard, PreflightOutcome::Clean));
        }

        let dirty = self.scanner.dirty_paths(&SCOPES)?;
        if dirty.is_empty() {
            self.manifest.write_manifest(&[])?;
            self.manifest.write_run_id(self.revision.as_deref())?;
            return Ok((guard, PreflightOutcome::Clean));
        }

        let manifest_paths = self.manifest.manifest()?;
        let run_id = self.manifest.run_id()?;
        let base_matches = matches!(
            (&run_id, &self.revision),
            (Some(recorded), Some(current)) if recorded == current
        );
        let usable = manifest_paths.is_some() && run_id.is_some();

        let fully_owned = usable
            && base_matches
            && dirty.iter().all(|path| {
                !matches!(
                    classify(
                        path,
                        &self.runner,
                        manifest_paths.as_deref().unwrap_or(&[]),
                        true,
                    ),
                    Ownership::Foreign
                )
            });

        if !fully_owned {
            return Err(PreflightError::ForeignDirt);
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
}
