//! The local file ports a sync run writes through, grouped so a new file
//! concern changes one struct.

use corpus::lock::ExclusiveLock;
use corpus::store::AtomicWrite;
use corpus::store::FileRemove;
use corpus::store::RecoveryCopies;
use vcs::tracking::RepositoryTracking;

/// A store a sync run writes and removes its files through, bounded by one
/// root.
pub trait SyncStore: AtomicWrite + FileRemove {}

impl<T: AtomicWrite + FileRemove + ?Sized> SyncStore for T {}

#[derive(Clone, Copy)]
pub struct SyncFiles<'a> {
    pub work_items: &'a dyn SyncStore,
    pub integration_state: &'a dyn SyncStore,
    pub file_locks: &'a dyn ExclusiveLock,
    pub recovery: &'a dyn RecoveryCopies,
    pub tracking: &'a dyn RepositoryTracking,
}
