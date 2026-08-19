//! Directory-tree artifact resolution: the adapter behind the tree ports.
//!
//! The orchestration over the leaf modules — `acquire`/`query` for the hit path,
//! `materialise` for the cold path, `verify` for the diagnostic walk. Trees are
//! deliberately not routed through `ResolveBinary::resolve`, whose per-exec
//! re-verify is precisely what they are exempt from.

pub mod attestation;
pub mod download;
pub mod extract;
pub mod layout;
pub mod lease;
pub mod pins;
pub mod reap;
pub mod seal;
pub mod table;

#[cfg(unix)]
mod resolver;
#[cfg(unix)]
pub use resolver::{ExpectedDigests, TreeResolver};
