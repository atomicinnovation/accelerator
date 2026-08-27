//! Persisting the discovery caches.
//!
//! Atomic writes, the advisory lock, and the `.gitignore` / `.gitkeep`
//! scaffold upkeep.
//!
//! The write path takes an injected [`Filesystem`] so its logic — atomicity,
//! lock-before-write, idempotent scaffold — is testable against a fake that can
//! fail a write mid-flight or present a held lock. The production
//! [`SystemFilesystem`] reuses the workspace's one atomic-write primitive and
//! its one mkdir-lock rather than a second implementation of either.

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

/// The gitignored entries in a Jira state directory.
const GITIGNORE_RULES: &[&str] = &[
    "site.json",
    ".refresh-meta.json",
    ".cache-version.json",
    ".lock/",
];

/// The lock directory name.
const LOCK_DIR: &str = ".lock";

/// The cache-layout version the binary writes and reads. Bash-era caches carry
/// no marker and are classified as the implicit version 0 — read unchanged.
const CACHE_VERSION: u64 = 1;

/// The marker file recording the cache-layout version.
const VERSION_FILE: &str = ".cache-version.json";

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("E_CACHE_IO: {path}: {detail}")]
    Io { path: String, detail: String },
    #[error("E_CACHE_LOCKED: another writer holds {path}")]
    LockContended { path: String },
    #[error("E_CACHE_BAD_JSON: {detail}")]
    Serialise { detail: String },
    /// The cache carries an unrecognised version marker, or a shape that does
    /// not parse as the known layout — a stale or corrupt cache that could feed
    /// wrong values into a live mutation. Read fails closed rather than trusting
    /// it.
    #[error(
        "E_CACHE_INCOMPATIBLE: {detail}; re-run init to rebuild the cache"
    )]
    Incompatible { detail: String },
}

/// The filesystem operations the cache writer needs.
pub trait Filesystem {
    fn exists(&self, path: &Path) -> bool;
    fn read(&self, path: &Path) -> Option<String>;
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

/// Writes a provider's discovery caches into one state directory.
pub struct JiraCache<'a> {
    fs: &'a dyn Filesystem,
    state_dir: PathBuf,
}

impl<'a> JiraCache<'a> {
    #[must_use]
    pub fn new(fs: &'a dyn Filesystem, state_dir: PathBuf) -> Self {
        Self { fs, state_dir }
    }

    /// Writes `site.json` and refreshes the scaffold.
    ///
    /// # Errors
    ///
    /// [`CacheError`] for a write or serialisation failure.
    pub fn write_site(&self, shape: &Value) -> Result<(), CacheError> {
        self.write_json("site.json", shape)?;
        self.stamp_version()?;
        self.ensure_scaffold()
    }

    /// Writes `projects.json` and `fields.json` under one lock.
    ///
    /// # Errors
    ///
    /// [`CacheError`] for contention, a write failure, or serialisation.
    pub fn write_discovery(
        &self,
        projects: &Value,
        fields: &Value,
    ) -> Result<(), CacheError> {
        let lockdir = self.state_dir.join(LOCK_DIR);
        self.fs.with_lock(&lockdir, &mut || {
            self.write_json("projects.json", projects)?;
            self.write_json("fields.json", fields)?;
            self.stamp_version()
        })
    }

    /// Reads a discovery cache file, failing closed on an unrecognised version
    /// marker or a shape that does not parse. An absent marker is
    /// the implicit bash-era version and reads unchanged, so no existing install
    /// must re-initialise.
    ///
    /// # Errors
    ///
    /// [`CacheError::Incompatible`] for a stale/corrupt cache; [`CacheError::Io`]
    /// when `name` is absent.
    pub fn read_cache(&self, name: &str) -> Result<Value, CacheError> {
        self.assert_version()?;
        let path = self.state_dir.join(name);
        let text = self.fs.read(&path).ok_or_else(|| CacheError::Io {
            path: path.display().to_string(),
            detail: "the cache file is absent".to_owned(),
        })?;
        serde_json::from_str(&text).map_err(|error| CacheError::Incompatible {
            detail: format!("{name} is not valid JSON: {error}"),
        })
    }

    /// Records the current cache-layout version, so a subsequent read is
    /// versioned rather than treated as bash-era.
    fn stamp_version(&self) -> Result<(), CacheError> {
        self.write_json(
            VERSION_FILE,
            &serde_json::json!({"version": CACHE_VERSION}),
        )
    }

    /// Classifies the version marker: absent is bash-era (permitted), the
    /// current version is permitted, anything else fails closed.
    fn assert_version(&self) -> Result<(), CacheError> {
        let Some(text) = self.fs.read(&self.state_dir.join(VERSION_FILE))
        else {
            return Ok(());
        };
        let marker: Value = serde_json::from_str(&text).map_err(|error| {
            CacheError::Incompatible {
                detail: format!(
                    "the version marker is not valid JSON: {error}"
                ),
            }
        })?;
        match marker.get("version").and_then(Value::as_u64) {
            Some(CACHE_VERSION) => Ok(()),
            other => Err(CacheError::Incompatible {
                detail: format!(
                    "cache version {other:?} is not the supported version \
                     {CACHE_VERSION}"
                ),
            }),
        }
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

    fn ensure_scaffold(&self) -> Result<(), CacheError> {
        let gitignore = self.state_dir.join(".gitignore");
        let existing = self.fs.read(&gitignore).unwrap_or_default();
        for rule in GITIGNORE_RULES {
            if !existing.lines().any(|line| line == *rule) {
                self.fs.append_line(&gitignore, rule)?;
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
/// the workspace's one mkdir-lock. Reusing both rather than reimplementing them
/// is what the store-duplication guard enforces.
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

    fn read(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
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
        // A whole-file rewrite through the same atomic primitive rather than an
        // append, so the scaffold never has a torn `.gitignore`.
        let mut content = self.read(path).unwrap_or_default();
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
        // `_guard` drops here, removing the lockdir.
    }
}
