//! Locally-declared outbound ports satisfied by `migrate-adapters`.
//!
//! `migrate` itself never imports `config`/`vcs`/`store` directly — every
//! capability a migration or the lifecycle engine needs is reached through
//! one of these traits.

use std::fmt;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationError(pub String);

impl MigrationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MigrationError {}

impl From<MigrationError> for kernel::Error {
    fn from(error: MigrationError) -> Self {
        Self::Failed(error.0)
    }
}

/// `migrate`'s own shape, mirroring `config::paths::DocTypeDir`'s fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocTypeDir {
    pub doc_type: String,
    pub dir: PathBuf,
}

/// The target-existence-check port migration 0007 needs.
///
/// Declared here, alongside `MigrationContext`'s other capabilities, rather
/// than as `corpus::CorpusIndex` — no such type exists in `corpus`.
pub trait CorpusIndex {
    fn target_exists(&self, target_type: &str, target_id: &str) -> bool;
}

/// The capabilities every migration's `apply()`/`apply_decision()`
/// implementation receives.
///
/// `write` is the only mutation path available to a migration — routing
/// every write through one method is what lets the path manifest be recorded
/// as a side effect of the call itself, rather than as a discipline each
/// migration author must remember.
pub trait MigrationContext {
    fn doc_type_dirs(&self) -> Vec<DocTypeDir>;
    fn revision(&self) -> Option<String>;
    fn corpus_index(&self) -> &dyn CorpusIndex;

    /// # Errors
    /// [`MigrationError`] when the write is refused or fails.
    fn write(&self, path: &Path, content: &str) -> Result<(), MigrationError>;
}

/// The applied/skipped ledger's file-backed persistence.
///
/// Ledger files are runner-managed bookkeeping — distinct from
/// `MigrationContext::write`'s manifest-tracked mutation path.
pub trait LedgerStore {
    /// # Errors
    /// [`MigrationError`] when the ledger cannot be read.
    fn applied(&self) -> Result<Vec<String>, MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the ledger cannot be written.
    fn write_applied(&self, ids: &[String]) -> Result<(), MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the skip list cannot be read.
    fn skipped(&self) -> Result<Vec<String>, MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the skip list cannot be written.
    fn write_skipped(&self, ids: &[String]) -> Result<(), MigrationError>;
}

pub struct PreviewEntry<'a> {
    pub id: &'a str,
    pub description: &'a str,
}

/// Reports lifecycle events for the caller to render.
///
/// Every user-facing string this engine emits is rendered by the caller
/// (`migrate-cli`'s `render` module owns the exact bash-parity literals) —
/// `migrate` itself only reports *which* event happened and in what order,
/// preserving bash's interleaved stdout/stderr sequencing without the domain
/// crate depending on `migrate-cli`.
pub trait Reporter {
    fn preview(&self, pending: &[PreviewEntry<'_>]);
    fn no_pending_migrations(&self, skipped: &[String]);
    fn unknown_applied_id(&self, id: &str);
    fn unknown_skipped_id(&self, id: &str);
    fn applied_and_skipped(&self, id: &str);
    fn migration_running(&self, id: &str);
    fn migration_applied(&self, id: &str);
    fn migration_no_op(&self, id: &str);
    fn migration_failed(&self, id: &str, error: &MigrationError);
    fn summary(
        &self,
        applied: usize,
        skipped: &[String],
        pending_remaining: usize,
    );
}
