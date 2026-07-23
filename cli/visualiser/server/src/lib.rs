//! Meta visualiser server — library crate.
//!
//! The binary (`src/main.rs`) is a thin entry point; all logic
//! lives in the modules declared here. Integration tests under
//! `server/tests/*.rs` consume these modules directly.

// A panic in a request or SSE-write handler crashes this crate's single
// long-running HTTP daemon, so production code is held to the workspace
// restriction policy. Scoped to non-test builds so test assertions may still
// use unwrap/expect; the few production sites carry per-occurrence #[allow].
#![cfg_attr(
    not(test),
    warn(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod activity;
pub mod activity_feed;
pub mod api;
pub mod assets;
pub mod cluster_key;
pub mod clusters;
pub mod config;
pub mod docs;
pub mod file_driver;
pub mod frontmatter;
pub mod indexer;
pub mod lifecycle;
pub mod log;
pub mod patcher;
pub mod related;
pub mod server;
pub mod shutdown;
pub mod slug;
pub mod sse_hub;
pub mod templates;
pub mod typed_ref;
pub mod watcher;
pub mod write_coordinator;

#[cfg(test)]
pub(crate) mod test_support;
