//! Test machinery shared by the provider sub-binaries' test lanes.
//!
//! Homes the reusable pieces that would otherwise be mirrored across
//! `linear-cli` and `jira-cli`: the `exit_codes.rs` textual parser (so a parity
//! test reads the constants the same way in both crates) and the scenario-JSON
//! loader that turns a retired bash mock's expectation set into
//! `http-test-support` routes.

pub mod exit_codes;
pub mod scenario;

pub use crate::exit_codes::parse_u8_consts;
pub use crate::scenario::Scenario;
