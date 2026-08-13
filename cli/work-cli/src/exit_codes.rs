//! The whole binary's exit-code taxonomy, in one place.
//!
//! Naming the low codes here too — not only 70-73 — is what stops 4 and 5
//! joining the hand-rolled literals already scattered through `main.rs`.
//!
//! `for_tracker_error` is consumed by `--push` wiring, which lands in a
//! later commit. `#[allow(dead_code)]` marks it until then rather than
//! leaving the taxonomy split across two definitions.

use tracker::TrackerError;

pub const CLEAN: u8 = 0;
pub const ERROR: u8 = 1;
pub const USAGE: u8 = 2;
pub const RESOLVE_NOT_FOUND: u8 = 3;
pub const UNRESOLVED: u8 = 4;
pub const REFUSED_BULK_OVERWRITE: u8 = 5;

pub const RETRYABLE: u8 = 70;
pub const TERMINAL: u8 = 71;
pub const NOT_AVAILABLE: u8 = 72;
pub const UNRECOGNISED: u8 = 73;

#[must_use]
#[allow(dead_code)]
pub const fn for_tracker_error(error: &TrackerError) -> u8 {
    match error {
        TrackerError::Retryable { .. } => RETRYABLE,
        TrackerError::Terminal { .. } => TERMINAL,
    }
}
