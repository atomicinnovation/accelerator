//! The compile-time, sorted-by-ID list of registered migrations.
//!
//! The Rust equivalent of `find ... | sort`, with no filesystem globbing: a
//! migration becomes reachable by adding a variant here, not by dropping a
//! script into a directory. `ACCELERATOR_MIGRATIONS_DIR` becomes moot — there
//! is no directory to override, since migrations are compiled in.

use crate::ports::MigrationContext;
use crate::ports::MigrationError;

pub trait MigrationMeta {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
}

pub enum ApplyOutcome {
    Applied,
    NoOpPending,
}

pub trait Migration: MigrationMeta {
    /// # Errors
    /// [`MigrationError`] when the migration cannot complete.
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError>;
}

/// One registered migration, dispatched by kind rather than downcast.
///
/// Only `Mechanical` exists so far — the `Interactive` variant is added once
/// `InteractiveMigration` (the Interactive Framework phase) exists to name it.
pub enum MigrationEntry {
    Mechanical(Box<dyn Migration>),
}

impl MigrationEntry {
    #[must_use]
    pub fn id(&self) -> &'static str {
        match self {
            Self::Mechanical(migration) => migration.id(),
        }
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Mechanical(migration) => migration.description(),
        }
    }
}

/// The fixed, sorted-by-ID list of registered migrations.
#[must_use]
pub const fn registry() -> Vec<MigrationEntry> {
    Vec::new()
}
