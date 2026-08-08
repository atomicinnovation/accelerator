//! The meta-directory migration engine's domain: the compiled-in registry,
//! the applied/skipped ledger, the path manifest, and the interactive
//! transformation contract. No filesystem, no subprocess — outbound access
//! goes through locally-declared port traits an adapter crate satisfies.

pub mod engine;
pub mod interactive;
pub mod ledger;
pub mod lifecycle;
pub mod manifest;
pub mod ports;
pub mod preflight;
pub mod registry;
