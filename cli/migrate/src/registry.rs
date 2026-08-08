//! The compile-time, sorted-by-ID list of registered migrations.
//!
//! The Rust equivalent of `find ... | sort`, with no filesystem globbing: a
//! migration becomes reachable by adding a variant here, not by dropping a
//! script into a directory.

/// One registered migration, dispatched by kind rather than downcast.
pub enum MigrationEntry {}

/// The fixed, sorted-by-ID list of registered migrations.
#[must_use]
pub const fn registry() -> Vec<MigrationEntry> {
    Vec::new()
}
