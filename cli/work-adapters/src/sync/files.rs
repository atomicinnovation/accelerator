//! The local file ports a sync run writes through, grouped so a new file
//! concern changes one struct.

use corpus::lock::ExclusiveLock;
use corpus::store::AtomicWrite;
use corpus::store::RecoveryCopies;
use vcs::tracking::RepositoryTracking;

#[derive(Clone, Copy)]
pub struct SyncFiles<'a> {
    pub work_items: &'a dyn AtomicWrite,
    pub integration_state: &'a dyn AtomicWrite,
    pub file_locks: &'a dyn ExclusiveLock,
    pub recovery: &'a dyn RecoveryCopies,
    pub tracking: &'a dyn RepositoryTracking,
}
