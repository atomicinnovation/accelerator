//! The research bounded context: asking scholarly sources for works and
//! deriving each work's reputation tier, planning which (focus area, profile)
//! pairs a topic's `conduct` round researches, coordinating the agents a
//! `conduct` run spawns, and confining those agents.
//!
//! Pure logic only. HTTP, JSON and XML decoding, the pacing lock and the
//! clock live behind ports implemented in `research-adapters`.

pub mod conduct;
pub mod confinement;
pub mod sources;
pub mod topic;
