//! The identity-run lock as a lockdir under `.accelerator/state/locks/`,
//! outside every corpus root, carrying the command that holds it.

use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use corpus::store::AtomicWrite;
use corpus::StoreError;
use store::lock::holder_pid;
use store::lock::LockError;
use store::lock::LockGuard;
use store::lock::LockOptions;
use work::rekey::outcome::Holder;

use crate::ignored_dir::prepare_ignored_dir;
use work::identity_run::HeldIdentityRun;
use work::identity_run::IdentityRun;
use work::identity_run::IdentityRunAttempt;
use work::identity_run::IdentityRunLockError;

const LOCKDIR: &str = "identity-run.lockdir";

/// Beside the store's `owner.<nonce>` sentinel, which is all the store
/// reads, and removed with the lockdir on release or reclaim.
const COMMAND_FILE: &str = "command";

pub struct IdentityRunLock<'a> {
    locks: PathBuf,
    writer: &'a dyn AtomicWrite,
}

impl<'a> IdentityRunLock<'a> {
    #[must_use]
    pub fn new(state_dir: &Path, writer: &'a dyn AtomicWrite) -> Self {
        Self {
            locks: state_dir.join("locks"),
            writer,
        }
    }

    fn lockdir(&self) -> PathBuf {
        self.locks.join(LOCKDIR)
    }

    fn holder(&self) -> Option<Holder> {
        let lockdir = self.lockdir();
        Some(Holder {
            pid: holder_pid(&lockdir)?,
            command: std::fs::read_to_string(lockdir.join(COMMAND_FILE)).ok(),
        })
    }

    fn acquire(
        &self,
        command: &str,
        ceiling: Duration,
    ) -> Result<Option<HeldIdentityRun>, StoreError> {
        prepare_ignored_dir(&self.locks, self.writer)?;
        let options = LockOptions {
            ceiling_ms: u64::try_from(ceiling.as_millis()).unwrap_or(u64::MAX),
            base_ms: 4,
            cap_ms: 256,
        };
        match store::lock::acquire(&self.lockdir(), options) {
            Ok(guard) => self.claimed_by(command, guard).map(Some),
            Err(LockError::Timeout { .. }) => Ok(None),
            Err(LockError::NotWritable { path }) => {
                Err(StoreError::NotWritable { path })
            }
            Err(LockError::Io { path, detail }) => {
                Err(StoreError::Io { path, detail })
            }
        }
    }

    fn claimed_by(
        &self,
        command: &str,
        guard: LockGuard,
    ) -> Result<HeldIdentityRun, StoreError> {
        let path = self.lockdir().join(COMMAND_FILE);
        std::fs::write(&path, command).map_err(|error| StoreError::Io {
            path: path.display().to_string(),
            detail: error.to_string(),
        })?;
        Ok(HeldIdentityRun::new(guard))
    }
}

impl IdentityRun for IdentityRunLock<'_> {
    fn try_acquire(
        &self,
        command: &str,
    ) -> Result<IdentityRunAttempt, StoreError> {
        Ok(self.acquire(command, Duration::ZERO)?.map_or_else(
            || IdentityRunAttempt::HeldBy(self.holder()),
            IdentityRunAttempt::Acquired,
        ))
    }

    fn acquire_within(
        &self,
        command: &str,
        ceiling: Duration,
    ) -> Result<HeldIdentityRun, IdentityRunLockError> {
        self.acquire(command, ceiling)
            .map_err(IdentityRunLockError::Lock)?
            .ok_or_else(|| IdentityRunLockError::Timeout(self.holder()))
    }
}
