//! A corpus store that writes as `FileCorpusStore` does, contained, staged
//! and published by rename, but never fsyncs.
//!
//! A test halts a run by unwinding, never by crashing the OS, so durability
//! buys it nothing. On macOS each sync is a full drive flush, which
//! serialises every test that writes in parallel.

use std::path::Path;
use std::path::PathBuf;

use corpus::store::ExclusiveCreate;
use corpus::AtomicWrite;
use corpus::FileRemove;
use corpus::StoreError;
use corpus_adapters::store::to_store_error;
use corpus_adapters::FileCorpusStore;
use store::NewFileMode;
use store::WriteBounds;

pub struct UnsyncedStore {
    root: PathBuf,
    remover: FileCorpusStore,
    fresh_mode: u32,
}

impl UnsyncedStore {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            remover: FileCorpusStore::new(root),
            fresh_mode: 0o666 & !store::current_umask(),
        }
    }

    fn bounds(&self) -> WriteBounds<'_> {
        WriteBounds {
            permitted_root: self.root.as_path(),
            project_root: self.root.as_path(),
        }
    }
}

impl AtomicWrite for UnsyncedStore {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        store::replace_without_sync(
            path,
            bytes,
            &self.bounds(),
            NewFileMode::PreserveOr(self.fresh_mode),
        )
        .map_err(to_store_error)
    }
}

impl ExclusiveCreate for UnsyncedStore {
    fn create_new(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        store::create_without_sync(
            path,
            bytes,
            &self.bounds(),
            NewFileMode::Set(self.fresh_mode),
        )
        .map_err(to_store_error)
    }
}

impl FileRemove for UnsyncedStore {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        self.remover.remove(path)
    }
}
