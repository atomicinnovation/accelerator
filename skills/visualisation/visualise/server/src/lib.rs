//! Meta visualiser server — library crate.
//!
//! The binary (`src/main.rs`) is a thin entry point; all logic
//! lives in the modules declared here. Integration tests under
//! `server/tests/*.rs` consume these modules directly.

pub mod activity;
pub mod api;
pub mod clusters;
pub mod config;
pub mod docs;
pub mod file_driver;
pub mod frontmatter;
pub mod indexer;
pub mod lifecycle;
pub mod server;
pub mod shutdown;
pub mod slug;
pub mod sse_hub;
pub mod templates;
pub mod watcher;
