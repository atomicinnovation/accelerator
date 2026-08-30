//! The compile-time, sorted-by-ID list of registered migrations.
//!
//! The Rust equivalent of `find ... | sort`, with no filesystem globbing: a
//! migration becomes reachable by adding a variant here, not by dropping a
//! script into a directory. `ACCELERATOR_MIGRATIONS_DIR` becomes moot — there
//! is no directory to override, since migrations are compiled in.

use crate::interactive::InteractiveMigration;
use crate::migrations::m0001::Migration0001;
use crate::migrations::m0002::Migration0002;
use crate::migrations::m0003::Migration0003;
use crate::migrations::m0004::Migration0004;
use crate::migrations::m0005::Migration0005;
use crate::migrations::m0006::Migration0006;
use crate::migrations::m0007::Migration0007;
use crate::migrations::m0008::Migration0008;
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
/// This is what routes migration 0007 (the only `Interactive` entry)
/// through `run_interactive()` instead of the mechanical apply loop's
/// `.apply(ctx)`.
pub enum MigrationEntry {
    Mechanical(Box<dyn Migration>),
    Interactive(Box<dyn InteractiveMigration>),
}

impl MigrationEntry {
    #[must_use]
    pub fn id(&self) -> &'static str {
        match self {
            Self::Mechanical(migration) => migration.id(),
            Self::Interactive(migration) => migration.id(),
        }
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Mechanical(migration) => migration.description(),
            Self::Interactive(migration) => migration.description(),
        }
    }
}

/// The fixed, sorted-by-ID list of registered migrations.
#[must_use]
pub fn registry() -> Vec<MigrationEntry> {
    vec![
        MigrationEntry::Mechanical(Box::new(Migration0001)),
        MigrationEntry::Mechanical(Box::new(Migration0002)),
        MigrationEntry::Mechanical(Box::new(Migration0003)),
        MigrationEntry::Mechanical(Box::new(Migration0004)),
        MigrationEntry::Mechanical(Box::new(Migration0005)),
        MigrationEntry::Mechanical(Box::new(Migration0006)),
        MigrationEntry::Interactive(Box::new(Migration0007)),
        MigrationEntry::Mechanical(Box::new(Migration0008)),
    ]
}
