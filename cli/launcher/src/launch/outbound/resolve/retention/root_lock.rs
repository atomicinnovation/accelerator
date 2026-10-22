//! The lock serialising root prunes of one cache root.
//!
//! It is released before any tree lock is taken or waited on, and a holder of
//! a tree lock only ever tries it, so the two never deadlock.

use std::io;
use std::path::Path;

use crate::launch::core::tree::TreeError;

use super::super::filesystem::FileKind;
use super::super::prune_report::PruneStep;
use super::super::single_flight::{LockGuard, LockPatience};
use super::ports::CachePorts;

pub const ROOT_LOCK: &str = ".accelerator-prune.lock";

pub struct HeldRootLock {
    _guard: LockGuard,
}

impl HeldRootLock {
    /// Take `root`'s prune lock, waiting or only trying; `None` when another
    /// prune holds it and `patience` is to try.
    ///
    /// A lock file that is a symlink or not this user's own can only be the
    /// same user's doing in a root nobody else can write to, so it is refused
    /// rather than followed or unlinked.
    ///
    /// # Errors
    ///
    /// [`TreeError::Lease`] if the lock file is not this user's regular file,
    /// or cannot be opened or locked.
    pub fn acquire(
        root: &Path,
        patience: LockPatience,
        ports: &CachePorts,
    ) -> Result<Option<Self>, TreeError> {
        let path = root.join(ROOT_LOCK);
        refuse_unless_owned_file(&path, ports)?;
        let guard = match patience {
            LockPatience::Wait => Some(ports.locks.take(&path)?),
            LockPatience::Try => ports.locks.try_take(&path)?,
        };
        Ok(guard.map(|guard| {
            ports.steps.reached(PruneStep::RootLockHeld);
            Self { _guard: guard }
        }))
    }
}

fn refuse_unless_owned_file(
    path: &Path,
    ports: &CachePorts,
) -> Result<(), TreeError> {
    let refused = |detail: String| TreeError::Lease {
        detail: format!("{}: {detail}", path.display()),
    };
    match ports.filesystem.entry(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(refused(format!("cannot examine the lock: {error}"))),
        Ok(entry)
            if entry.kind == FileKind::File
                && entry.owner == ports.ownership.effective_uid() =>
        {
            Ok(())
        }
        Ok(_) => Err(refused(
            "the prune lock is not a regular file this user owns".to_owned(),
        )),
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use crate::launch::core::tree::TreeError;

    use super::super::super::filesystem::testing::ReportingFilesystem;
    use super::super::super::filesystem::{
        CacheFilesystem, EntryMetadata, SystemFilesystem,
    };
    use super::super::super::ownership::{Ownership, ProcessOwnership};
    use super::super::super::prune_report::{PruneStep, PruneSteps};
    use super::super::super::single_flight::{LockPatience, SingleFlightLocks};
    use super::super::super::tree::locks::FlockLocks;
    use super::super::super::tree::testing::{within, BLOCKING_BOUND};
    use super::super::ports::CachePorts;
    use super::super::testing::Ports;
    use super::{HeldRootLock, ROOT_LOCK};

    #[derive(Default)]
    struct SeenSteps(Mutex<Vec<PruneStep>>);

    impl PruneSteps for SeenSteps {
        fn reached(&self, step: PruneStep) {
            self.0.lock().expect("steps").push(step);
        }
    }

    fn ports_over(
        filesystem: Arc<dyn CacheFilesystem + Send + Sync>,
        steps: Arc<SeenSteps>,
    ) -> Arc<CachePorts> {
        Arc::new(
            Ports {
                filesystem,
                steps,
                ..Ports::at(0)
            }
            .build(),
        )
    }

    fn ports() -> Arc<CachePorts> {
        ports_over(Arc::new(SystemFilesystem), Arc::default())
    }

    fn acquired(
        root: &Path,
        patience: LockPatience,
        ports: Arc<CachePorts>,
    ) -> Result<bool, TreeError> {
        let root = root.to_path_buf();
        within(BLOCKING_BOUND, "the root lock", move || {
            HeldRootLock::acquire(&root, patience, &ports)
                .map(|held| held.is_some())
        })
    }

    #[test]
    fn a_free_root_lock_is_taken_and_the_step_reported() {
        let root = tempfile::tempdir().expect("root");
        let steps = Arc::new(SeenSteps::default());
        let ports = ports_over(Arc::new(SystemFilesystem), Arc::clone(&steps));

        assert!(matches!(
            acquired(root.path(), LockPatience::Wait, ports),
            Ok(true)
        ));
        assert_eq!(*steps.0.lock().expect("steps"), [PruneStep::RootLockHeld]);
    }

    #[test]
    fn a_held_root_lock_is_skipped_when_only_tried() {
        let root = tempfile::tempdir().expect("root");
        let _elsewhere =
            FlockLocks.take(&root.path().join(ROOT_LOCK)).expect("held");

        assert!(matches!(
            acquired(root.path(), LockPatience::Try, ports()),
            Ok(false)
        ));
    }

    #[test]
    fn a_symlinked_root_lock_is_refused_and_its_target_untouched() {
        let root = tempfile::tempdir().expect("root");
        let target = root.path().join("target");
        std::fs::write(&target, b"sentinel").expect("target");
        std::os::unix::fs::symlink(&target, root.path().join(ROOT_LOCK))
            .expect("symlink");

        for patience in [LockPatience::Wait, LockPatience::Try] {
            assert!(matches!(
                acquired(root.path(), patience, ports()),
                Err(TreeError::Lease { .. })
            ));
        }
        assert_eq!(std::fs::read(&target).expect("target"), b"sentinel");
    }

    #[test]
    fn a_foreign_owned_root_lock_is_refused() {
        let root = tempfile::tempdir().expect("root");
        let lock = root.path().join(ROOT_LOCK);
        std::fs::write(&lock, b"").expect("lock");
        let filesystem = ReportingFilesystem::over_system().reporting(
            &lock,
            EntryMetadata {
                owner: ProcessOwnership::default().effective_uid() + 1,
                ..SystemFilesystem.entry(&lock).expect("lock metadata")
            },
        );

        assert!(matches!(
            acquired(
                root.path(),
                LockPatience::Wait,
                ports_over(Arc::new(filesystem), Arc::default())
            ),
            Err(TreeError::Lease { .. })
        ));
    }
}
