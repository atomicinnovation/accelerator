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
pub mod record;
pub mod request;
pub mod round;
pub mod schedule;
pub mod tier;
