//! The production single-flight locks, over `flock`.

use std::path::Path;

use crate::launch::core::tree::TreeError;

use super::super::single_flight::{LockGuard, LockProbe, SingleFlightLocks};
use super::lease::{
    probe_single_flight, take_single_flight, try_single_flight, wait_existing,
    SingleFlightProbe,
};

pub struct FlockLocks;

impl SingleFlightLocks for FlockLocks {
    fn take(&self, path: &Path) -> Result<LockGuard, TreeError> {
        take_single_flight(path).map(LockGuard::holding)
    }

    fn try_take(&self, path: &Path) -> Result<Option<LockGuard>, TreeError> {
        Ok(try_single_flight(path)?.map(LockGuard::holding))
    }

    fn wait_existing(
        &self,
        path: &Path,
    ) -> Result<Option<LockGuard>, TreeError> {
        Ok(wait_existing(path)?.map(LockGuard::holding))
    }

    fn probe(&self, path: &Path) -> Result<LockProbe, TreeError> {
        Ok(match probe_single_flight(path)? {
            SingleFlightProbe::Taken(guard) => {
                LockProbe::Taken(LockGuard::holding(guard))
            }
            SingleFlightProbe::Held => LockProbe::Held,
            SingleFlightProbe::Absent => LockProbe::Absent,
        })
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::super::single_flight::{LockProbe, SingleFlightLocks};
    use super::super::testing::{within, BLOCKING_BOUND};
    use super::FlockLocks;

    #[test]
    fn a_taken_lock_is_held_until_its_guard_drops() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("lock");

        let guard = FlockLocks.take(&path).expect("taken");
        let probed = {
            let path = path.clone();
            within(BLOCKING_BOUND, "the probe", move || {
                matches!(FlockLocks.probe(&path), Ok(LockProbe::Held))
            })
        };
        assert!(probed, "the taken lock is not held");
        drop(guard);

        within(BLOCKING_BOUND, "the retake", move || FlockLocks.take(&path))
            .expect("retaken once released");
    }

    #[test]
    fn neither_waiting_nor_probing_creates_a_lock_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("lock");

        assert!(FlockLocks.wait_existing(&path).expect("waited").is_none());
        assert!(matches!(FlockLocks.probe(&path), Ok(LockProbe::Absent)));
        assert!(!path.exists());
    }
}
