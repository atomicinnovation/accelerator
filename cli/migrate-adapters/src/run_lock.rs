//! The run-level advisory lock.
//!
//! A whole-run mkdir-lock scoped to `.accelerator/state/`, sharing
//! `corpus-adapters::lock`'s primitive with a short, explicit ceiling — a
//! `run_pending()` invocation can legitimately span an interactive session,
//! so it must refuse promptly rather than silently block for
//! `LockOptions::default()`'s 5-minute ceiling.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use corpus::StoreError;
use corpus_adapters::lock;
use migrate::ports::MigrationError;
use migrate::ports::RunLock;
use migrate::ports::RunLockGuard;

const CEILING_MS: u64 = 2_000;

pub struct FileRunLock {
    lockdir: PathBuf,
}

impl FileRunLock {
    #[must_use]
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            lockdir: root
                .as_ref()
                .join(".accelerator/state/migrate-run.lockdir"),
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
            Err(StoreError::LockTimeout { .. }) => {
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

/// The single `owner.<nonce>` sentinel's PID, read directly since
/// `corpus_adapters::lock`'s own reader is private. `None` for anything
/// ambiguous — absent, more than one match, or unparseable — including the
/// narrow mid-reclaim window where only a `reclaiming.<pid>.<nonce>`
/// sentinel is present.
fn holder_pid(lockdir: &Path) -> Option<i32> {
    let mut found = None;
    for entry in fs::read_dir(lockdir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("owner.") {
            if found.is_some() {
                return None;
            }
            found = Some(name);
        }
    }
    fs::read_to_string(lockdir.join(found?))
        .ok()?
        .trim()
        .parse()
        .ok()
}
