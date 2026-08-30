//! Outbound adapters for the `work` domain crate: filesystem reads,
//! in-process section diffing, and VCS-derived authorship.

pub mod author;
pub mod diff;
pub mod filesystem;
pub mod sync;
