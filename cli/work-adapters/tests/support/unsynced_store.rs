//! A corpus store that writes as `FileCorpusStore` does, contained, staged
//! and published by rename, but never fsyncs.
//!
//! A test halts a run by unwinding, never by crashing the OS, so durability
//! buys it nothing. On macOS each sync is a full drive flush, which
//! serialises every test that writes in parallel.

use std::fs;
use std::io::ErrorKind;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::path::PathBuf;

use corpus::store::ExclusiveCreate;
use corpus::AtomicWrite;
use corpus::FileRemove;
use corpus::StoreError;
use corpus_adapters::FileCorpusStore;
use store::NewFileMode;
use store::WriteBounds;
use store::WriteError;

const TEMP_RANDOM_LEN: usize = 6;

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
        .map_err(refusal)
    }
}

impl ExclusiveCreate for UnsyncedStore {
    fn create_new(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        let parent =
            store::ensure_contained(path, &self.bounds()).map_err(refusal)?;
        fs::create_dir_all(&parent)
            .map_err(|error| failure(&parent, &error))?;
        let target = parent.join(path.file_name().ok_or_else(|| {
            StoreError::UnsafePath {
                path: path.display().to_string(),
            }
        })?);
        let mut staged = tempfile::Builder::new()
            .prefix(kernel::TEMP_PREFIX)
            .rand_bytes(TEMP_RANDOM_LEN)
            .tempfile_in(&parent)
            .map_err(|error| failure(&parent, &error))?;
        fs::set_permissions(
            staged.path(),
            fs::Permissions::from_mode(self.fresh_mode),
        )
        .map_err(|error| failure(staged.path(), &error))?;
        staged
            .write_all(bytes)
            .map_err(|error| failure(&parent, &error))?;
        staged
            .persist_noclobber(&target)
            .map(|_| ())
            .map_err(|error| failure(&target, &error.error))
    }
}

impl FileRemove for UnsyncedStore {
    fn remove(&self, path: &Path) -> Result<(), StoreError> {
        self.remover.remove(path)
    }
}

fn failure(path: &Path, error: &std::io::Error) -> StoreError {
    let path = path.display().to_string();
    match error.kind() {
        ErrorKind::AlreadyExists => StoreError::AlreadyExists { path },
        ErrorKind::PermissionDenied => StoreError::NotWritable {
            path,
            detail: error.to_string(),
        },
        _ => StoreError::Io {
            path,
            detail: error.to_string(),
        },
    }
}

fn refusal(error: WriteError) -> StoreError {
    match error {
        WriteError::NotWritable { path, detail } => {
            StoreError::NotWritable { path, detail }
        }
        WriteError::CrossFilesystem { path } => {
            StoreError::CrossFilesystem { path }
        }
        WriteError::UnsafePath { path } => StoreError::UnsafePath { path },
        WriteError::AlreadyExists { path } => {
            StoreError::AlreadyExists { path }
        }
        WriteError::Io { path, detail } => StoreError::Io { path, detail },
        other => StoreError::Io {
            path: String::new(),
            detail: other.to_string(),
        },
    }
}
