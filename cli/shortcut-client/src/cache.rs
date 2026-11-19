//! Reading and writing `catalogue.json`.
//!
//! Only `init discover` writes it, through the store's one atomic write and
//! its one mkdir-lock, and only when every listing completed. The write path
//! takes an injected [`Filesystem`] for the same reason Linear's does.
//! Duplicated rather than shared with the sibling clients: they may not import
//! each other.

use std::path::Path;
use std::path::PathBuf;

use store::atomic_write;
use store::lock::acquire;
use store::lock::LockError;
use store::lock::LockOptions;
use store::NewFileMode;
use store::WriteBounds;
use thiserror::Error;

use crate::catalogue::Catalogue;
use crate::catalogue::CatalogueError;

/// The gitignored entries in the Shortcut state directory. `catalogue.json` is
/// absent on purpose: teams and workflows are shared, so it is committed.
const GITIGNORE_RULES: &[&str] =
    &["viewer.json", ".refresh-meta.json", ".lock/"];
const LOCK_DIR: &str = ".lock";
const CATALOGUE_FILE: &str = "catalogue.json";

/// Whether every listing a discovery ran reached its last page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listing {
    Complete,
    Incomplete { listing: String, cause: String },
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("E_CACHE_IO: {path}: {detail}")]
    Io { path: String, detail: String },
    #[error("E_CACHE_LOCKED: another writer holds {path}")]
    LockContended { path: String },
    #[error(
        "E_DISCOVER_INCOMPLETE: the {listing} listing stopped before its last \
         page ({cause}), so catalogue.json was left as it was; retry \
         `accelerator shortcut init discover`"
    )]
    Incomplete { listing: String, cause: String },
    #[error("{0}")]
    Catalogue(CatalogueError),
}

/// The filesystem operations the cache needs.
pub trait Filesystem {
    fn exists(&self, path: &Path) -> bool;
    /// # Errors
    ///
    /// [`CacheError::Io`] when the directory cannot be created.
    fn ensure_dir(&self, path: &Path) -> Result<(), CacheError>;
    /// # Errors
    ///
    /// [`CacheError::Io`] when the file exists but cannot be read.
    fn read(&self, path: &Path) -> Result<Option<String>, CacheError>;
    /// # Errors
    ///
    /// [`CacheError::Io`] when the write cannot complete.
    fn write_atomic(&self, path: &Path, bytes: &[u8])
        -> Result<(), CacheError>;
    /// # Errors
    ///
    /// [`CacheError::Io`] when the file cannot be created.
    fn ensure_present(&self, path: &Path) -> Result<(), CacheError>;
    /// # Errors
    ///
    /// [`CacheError::Io`] when the append fails.
    fn append_line(&self, path: &Path, line: &str) -> Result<(), CacheError>;
    /// # Errors
    ///
    /// [`CacheError::LockContended`] when the lock is held, or the error
    /// `body` itself returns.
    fn with_lock(
        &self,
        lockdir: &Path,
        body: &mut dyn FnMut() -> Result<(), CacheError>,
    ) -> Result<(), CacheError>;
}

pub struct ShortcutCache<'a> {
    fs: &'a dyn Filesystem,
    state_dir: PathBuf,
}

impl<'a> ShortcutCache<'a> {
    #[must_use]
    pub fn new(fs: &'a dyn Filesystem, state_dir: PathBuf) -> Self {
        Self { fs, state_dir }
    }

    /// The stored catalogue, or `None` when there is none.
    ///
    /// # Errors
    ///
    /// [`CacheError::Io`] when the file cannot be read, and
    /// [`CacheError::Catalogue`] when it is not a catalogue this client reads.
    pub fn load(&self) -> Result<Option<Catalogue>, CacheError> {
        self.fs
            .read(&self.catalogue_path())?
            .map(|text| Catalogue::parse(&text).map_err(CacheError::Catalogue))
            .transpose()
    }

    /// Writes `fresh` under the lock, keeping the fields the stored catalogue
    /// holds and `fresh` does not.
    ///
    /// # Errors
    ///
    /// [`CacheError::Incomplete`] for a listing that stopped short,
    /// [`CacheError::Catalogue`] when the stored catalogue is newer than this
    /// client, and [`CacheError`] for contention or a failed write.
    pub fn write(
        &self,
        fresh: Catalogue,
        listing: Listing,
    ) -> Result<(), CacheError> {
        if let Listing::Incomplete { listing, cause } = listing {
            return Err(CacheError::Incomplete { listing, cause });
        }
        self.fs.ensure_dir(&self.state_dir)?;
        let mut fresh = Some(fresh);
        self.fs.with_lock(&self.state_dir.join(LOCK_DIR), &mut || {
            let Some(catalogue) = fresh.take() else {
                return Ok(());
            };
            let merged = match self.load() {
                Ok(Some(stored)) => catalogue.merged_over(&stored),
                Err(CacheError::Catalogue(newer @ CatalogueError::NewerSchema { .. })) => {
                    return Err(CacheError::Catalogue(newer));
                }
                Ok(None) | Err(CacheError::Catalogue(_)) => catalogue,
                Err(other) => return Err(other),
            };
            let text = merged.to_json().map_err(CacheError::Catalogue)?;
            self.fs.write_atomic(&self.catalogue_path(), text.as_bytes())?;
            tracing::info!(path = %self.catalogue_path().display(), "wrote the shortcut catalogue");
            self.ensure_scaffold()
        })
    }

    fn catalogue_path(&self) -> PathBuf {
        self.state_dir.join(CATALOGUE_FILE)
    }

    fn ensure_scaffold(&self) -> Result<(), CacheError> {
        let gitignore = self.state_dir.join(".gitignore");
        match self.fs.read(&gitignore) {
            Ok(existing) => {
                let existing = existing.unwrap_or_default();
                for rule in GITIGNORE_RULES {
                    if !existing.lines().any(|line| line == *rule) {
                        self.fs.append_line(&gitignore, rule)?;
                    }
                }
            }
            Err(error) => tracing::warn!("skipping .gitignore upkeep: {error}"),
        }
        let gitkeep = self.state_dir.join(".gitkeep");
        if !self.fs.exists(&gitkeep) {
            self.fs.ensure_present(&gitkeep)?;
        }
        Ok(())
    }
}

/// The real filesystem, through the store's atomic write and mkdir-lock.
pub struct SystemFilesystem {
    project_root: PathBuf,
    lock_options: LockOptions,
}

impl SystemFilesystem {
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            lock_options: LockOptions::default(),
        }
    }

    #[must_use]
    pub const fn with_lock_options(mut self, options: LockOptions) -> Self {
        self.lock_options = options;
        self
    }
}

impl Filesystem for SystemFilesystem {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn ensure_dir(&self, path: &Path) -> Result<(), CacheError> {
        std::fs::create_dir_all(path).map_err(|error| CacheError::Io {
            path: path.display().to_string(),
            detail: error.to_string(),
        })
    }

    fn read(&self, path: &Path) -> Result<Option<String>, CacheError> {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(None)
            }
            Err(error) => Err(CacheError::Io {
                path: path.display().to_string(),
                detail: error.to_string(),
            }),
        }
    }

    fn write_atomic(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), CacheError> {
        let permitted = path.parent().unwrap_or_else(|| Path::new("."));
        atomic_write(
            path,
            bytes,
            &WriteBounds {
                permitted_root: permitted,
                project_root: &self.project_root,
            },
            NewFileMode::PreserveOr(0o644),
        )
        .map_err(|error| CacheError::Io {
            path: path.display().to_string(),
            detail: error.to_string(),
        })
    }

    fn ensure_present(&self, path: &Path) -> Result<(), CacheError> {
        if path.exists() {
            return Ok(());
        }
        self.write_atomic(path, b"")
    }

    fn append_line(&self, path: &Path, line: &str) -> Result<(), CacheError> {
        let mut content = self.read(path)?.unwrap_or_default();
        content.push_str(line);
        content.push('\n');
        self.write_atomic(path, content.as_bytes())
    }

    fn with_lock(
        &self,
        lockdir: &Path,
        body: &mut dyn FnMut() -> Result<(), CacheError>,
    ) -> Result<(), CacheError> {
        let _guard =
            acquire(lockdir, self.lock_options).map_err(
                |error| match error {
                    LockError::Timeout { path } => {
                        CacheError::LockContended { path }
                    }
                    error @ (LockError::NotWritable { .. }
                    | LockError::Io { .. }) => CacheError::Io {
                        path: lockdir.display().to_string(),
                        detail: error.to_string(),
                    },
                },
            )?;
        body()
    }
}
