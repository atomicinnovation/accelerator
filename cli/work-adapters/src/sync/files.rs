//! The local file ports a sync run writes through, grouped so a new file
//! concern changes one struct.

use corpus::lock::ExclusiveLock;
use corpus::store::AtomicWrite;

#[derive(Clone, Copy)]
pub struct SyncFiles<'a> {
    pub work_items: &'a dyn AtomicWrite,
    pub integration_state: &'a dyn AtomicWrite,
    pub file_locks: &'a dyn ExclusiveLock,
}
