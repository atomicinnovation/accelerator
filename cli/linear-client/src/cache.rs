//! Persisting the discovery caches.
//!
//! Atomic writes, the advisory lock, and the `.gitignore` / `.gitkeep`
//! scaffold upkeep — the Linear mirror of `jira-client`'s cache.
//!
//! The write path takes an injected [`Filesystem`] so its logic — atomicity,
//! lock-before-write, idempotent scaffold — is testable against a fake that can
//! fail a write mid-flight or present a held lock. The production
//! [`SystemFilesystem`] reuses the workspace's one atomic-write primitive and
//! its one mkdir-lock rather than a second implementation of either.
//!
//! Duplicated rather than shared with `jira-client`: the two clients may not
//! import each other, and the shape carries no provider specifics that would
//! drift.

use std::path::Path;
use std::path::PathBuf;

use corpus::StoreError;
use corpus_adapters::acquire;
use corpus_adapters::LockOptions;
use serde_json::Value;
use store::atomic_write;
use store::NewFileMode;
use store::WriteBounds;
use thiserror::Error;

use crate::catalogue::CatalogueDocument;
use crate::catalogue::CatalogueParseError;
use crate::catalogue::CatalogueUpdate;
use crate::catalogue::Strictness;

/// The gitignored entries in the Linear state directory. `catalogue.json` is
/// deliberately absent — team and states are team-scoped, not per-developer,
/// so it is committed.
const GITIGNORE_RULES: &[&str] =
    &["viewer.json", ".refresh-meta.json", ".lock/"];

/// The lock directory name guarding the catalogue write.
const LOCK_DIR: &str = ".lock";

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("E_CACHE_IO: {path}: {detail}")]
    Io { path: String, detail: String },
    #[error("E_CACHE_LOCKED: another writer holds {path}")]
    LockContended { path: String },
    #[error("E_CACHE_BAD_JSON: {detail}")]
    Serialise { detail: String },
    #[error(
        "E_CACHE_UNPARSEABLE: {path}: {reason}. Restore the last good \
         catalogue.json from version control, or resolve its merge conflict. \
         As a last resort, delete it and re-run init; synced teams are then \
         re-derived from tracked work items on the next apply sync"
    )]
    Unparseable {
        path: String,
        reason: CatalogueParseError,
    },
}

/// The filesystem operations the cache writer needs.
pub trait Filesystem {
    fn exists(&self, path: &Path) -> bool;
    /// The file's content, or `None` when it does not exist.
    ///
    /// # Errors
    ///
    /// [`CacheError::Io`] when the file exists but cannot be read.
    fn read(&self, path: &Path) -> Result<Option<String>, CacheError>;
    /// Writes `bytes` to `path` such that no reader ever observes a partial
    /// file: a mid-write failure leaves the previous content in place.
    ///
    /// # Errors
    ///
    /// [`CacheError::Io`] when the write cannot complete.
    fn write_atomic(&self, path: &Path, bytes: &[u8])
        -> Result<(), CacheError>;
    /// Ensures `path` exists, creating an empty file if absent.
    ///
    /// # Errors
    ///
    /// [`CacheError::Io`] when the file cannot be created.
    fn ensure_present(&self, path: &Path) -> Result<(), CacheError>;
    /// Appends `line` and a newline to `path`, creating it if absent.
    ///
    /// # Errors
    ///
    /// [`CacheError::Io`] when the append fails.
    fn append_line(&self, path: &Path, line: &str) -> Result<(), CacheError>;
    /// Runs `body` while holding the advisory lock on `lockdir`, releasing it
    /// afterwards. Returns [`CacheError::LockContended`] rather than clobbering
    /// a lock another writer holds.
    ///
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

/// Writes the Linear discovery caches into one state directory.
pub struct LinearCache<'a> {
    fs: &'a dyn Filesystem,
    state_dir: PathBuf,
}

impl<'a> LinearCache<'a> {
    #[must_use]
    pub fn new(fs: &'a dyn Filesystem, state_dir: PathBuf) -> Self {
        Self { fs, state_dir }
    }

    /// Writes `viewer.json` and refreshes the scaffold.
    ///
    /// # Errors
    ///
    /// [`CacheError`] for a write or serialisation failure.
    pub fn write_viewer(&self, shape: &Value) -> Result<(), CacheError> {
        self.write_json("viewer.json", shape)?;
        self.ensure_scaffold()
    }

    /// The stored catalogue, parsed as strictly as a writer must: an absent
    /// file is an empty catalogue, and any entry that would not survive a
    /// rewrite refuses the load.
    ///
    /// # Errors
    ///
    /// [`CacheError::Io`] when the file cannot be read, and
    /// [`CacheError::Unparseable`] when it cannot be parsed losslessly.
    pub fn load_for_update(&self) -> Result<CatalogueDocument, CacheError> {
        let path = self.catalogue_path();
        let Some(text) = self.fs.read(&path)? else {
            return Ok(CatalogueDocument::default());
        };
        CatalogueDocument::parse(&text, Strictness::Strict).map_err(|reason| {
            CacheError::Unparseable {
                path: path.display().to_string(),
                reason,
            }
        })
    }

    /// Merges `update` into `catalogue.json` under the advisory lock, so two
    /// concurrent writers cannot each read the pre-merge file and clobber one
    /// another. Returns the keys of teams catalogued for the first time.
    ///
    /// # Errors
    ///
    /// [`CacheError`] for contention, a read or write failure, or a stored
    /// catalogue that cannot be parsed losslessly.
    pub fn record_team_entries(
        &self,
        update: &CatalogueUpdate,
    ) -> Result<Vec<String>, CacheError> {
        let mut newly_catalogued = Vec::new();
        self.fs.with_lock(&self.state_dir.join(LOCK_DIR), &mut || {
            let mut document = self.load_for_update()?;
            newly_catalogued = document.record(update.clone());
            let text =
                document.to_json().map_err(|error| CacheError::Serialise {
                    detail: error.to_string(),
                })?;
            self.fs
                .write_atomic(&self.catalogue_path(), text.as_bytes())?;
            self.ensure_scaffold()
        })?;
        Ok(newly_catalogued)
    }

    fn catalogue_path(&self) -> PathBuf {
        self.state_dir.join("catalogue.json")
    }

    fn write_json(&self, name: &str, value: &Value) -> Result<(), CacheError> {
        let mut text =
            serde_json::to_string_pretty(value).map_err(|error| {
                CacheError::Serialise {
                    detail: error.to_string(),
                }
            })?;
        text.push('\n');
        self.fs
            .write_atomic(&self.state_dir.join(name), text.as_bytes())
    }

    /// Scaffold upkeep is incidental to the write that precedes it, so an
    /// unreadable `.gitignore` skips the appends rather than failing a write
    /// that already went ahead.
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
            Err(error) => {
                tracing::warn!("skipping .gitignore upkeep: {error}");
            }
        }
        let gitkeep = self.state_dir.join(".gitkeep");
        if !self.fs.exists(&gitkeep) {
            self.fs.ensure_present(&gitkeep)?;
        }
        Ok(())
    }
}

/// The real filesystem.
///
/// Whole-file writes go through the store's one `atomic_write` primitive and
/// the workspace's one mkdir-lock. Reusing both rather than reimplementing
/// them is what the store-duplication guard enforces.
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

    fn write_bounds<'a>(&'a self, permitted: &'a Path) -> WriteBounds<'a> {
        WriteBounds {
            permitted_root: permitted,
            project_root: &self.project_root,
        }
    }
}

impl Filesystem for SystemFilesystem {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
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
            &self.write_bounds(permitted),
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
                    StoreError::LockTimeout { path } => {
                        CacheError::LockContended { path }
                    }
                    other => CacheError::Io {
                        path: lockdir.display().to_string(),
                        detail: other.to_string(),
                    },
                },
            )?;
        body()
    }
}
