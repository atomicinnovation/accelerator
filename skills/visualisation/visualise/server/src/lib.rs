//! Meta visualiser server — library crate.
//!
//! The binary (`src/main.rs`) is a thin entry point; all logic
//! lives in the modules declared here. Integration tests under
//! `server/tests/*.rs` consume these modules directly.

pub mod config;

// Modules are added in later sub-phases:
// 2.3 → server
// 2.4 → shutdown  (ShutdownReason + signal handlers)
// 2.5 → activity, lifecycle
