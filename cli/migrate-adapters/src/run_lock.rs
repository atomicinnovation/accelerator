//! The run-level advisory lock.
//!
//! A whole-run mkdir-lock scoped to `.accelerator/state/`, sharing
//! `store::lock`'s primitive with a short, explicit ceiling — a
//! `run_pending()` invocation can legitimately span an interactive session,
//! so it must refuse promptly rather than silently block for
//! `LockOptions::default()`'s 5-minute ceiling.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use migrate::ports::MigrationError;
use migrate::ports::RunLock;
use migrate::ports::RunLockGuard;
use store::lock::{self, holder_pid, LockError};

const CEILING_MS: u64 = 2_000;

/// Repo-relative, so the pre-flight can recognise the lock it is itself
/// holding as its own rather than as dirt in the tree it is scanning.
pub const LOCK_DIR: &str = ".accelerator/state/migrate-run.lockdir";

pub struct FileRunLock {
    lockdir: PathBuf,
}

impl FileRunLock {
    #[must_use]
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            lockdir: root.as_ref().join(LOCK_DIR),
        }
    }
}

impl RunLock for FileRunLock {
    fn acquire(&self) -> Result<RunLockGuard, MigrationError> {
        if let Some(parent) = self.lockdir.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| MigrationError::new(error.to_string()))?;
        }
        let options = lock::LockOptions {
            ceiling_ms: CEILING_MS,
            base_ms: 4,
            cap_ms: 256,
        };
        match lock::acquire(&self.lockdir, options) {
            Ok(guard) => Ok(RunLockGuard::new(guard)),
            Err(LockError::Timeout { .. }) => {
                Err(MigrationError::new(refusal_message(&self.lockdir)))
            }
            Err(other) => Err(MigrationError::new(other.to_string())),
        }
    }
}

fn refusal_message(lockdir: &Path) -> String {
    holder_pid(lockdir).map_or_else(
        || {
            "Another accelerator migrate run is already in progress (pid \
             unknown, reclaim in progress)."
                .to_owned()
        },
        |pid| {
            format!(
                "Another accelerator migrate run is already in progress \
                 (pid {pid})."
            )
        },
    )
}
