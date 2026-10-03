//! The research bounded context: asking scholarly sources for works,
//! deriving each work's reputation tier deterministically, and planning which
//! (focus area, profile) pairs a topic's `conduct` round researches.
//!
//! Pure logic only. HTTP, JSON and XML decoding, the pacing lock and the
//! clock live behind ports implemented in `research-adapters`.

pub mod arxiv;
pub mod classify;
pub mod confinement;
pub mod fetch;
pub mod openalex;
pub mod pinned_indexes;
pub mod question;
pub mod record;
pub mod request;
pub mod round;
pub mod run_ledger;
pub mod schedule;
pub mod spawn_window;
pub mod tier;
pub mod tree;
