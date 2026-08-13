//! Persistence and apply for the remote sync engine: digest computation,
//! the baseline document, and the per-item apply sequence over
//! `tracker::RemoteTracker`.

pub mod apply;
pub mod baseline;
pub mod baseline_store;
pub mod digest;
pub mod fetch;
pub mod run;
