//! The whole binary's exit-code taxonomy, in one place.
//!
//! Naming the low codes here too — not only 70-73 — is what stops 4 and 5
//! joining the hand-rolled literals already scattered through `main.rs`.
//!
//! `UNRESOLVED`, `REFUSED_BULK_OVERWRITE`, the four dispatch codes and
//! `for_tracker_error` are consumed by the `sync` command and `--push`
//! wiring, which land in later commits. `#[allow(dead_code)]` marks them
//! until then rather than leaving the taxonomy split across two definitions.

use tracker::TrackerError;

#[allow(dead_code)]
pub const CLEAN: u8 = 0;
#[allow(dead_code)]
pub const ERROR: u8 = 1;
pub const USAGE: u8 = 2;
pub const RESOLVE_NOT_FOUND: u8 = 3;
#[allow(dead_code)]
pub const UNRESOLVED: u8 = 4;
#[allow(dead_code)]
pub const REFUSED_BULK_OVERWRITE: u8 = 5;

#[allow(dead_code)]
pub const RETRYABLE: u8 = 70;
#[allow(dead_code)]
pub const TERMINAL: u8 = 71;
#[allow(dead_code)]
pub const NOT_AVAILABLE: u8 = 72;
#[allow(dead_code)]
pub const UNRECOGNISED: u8 = 73;

#[must_use]
#[allow(dead_code)]
pub const fn for_tracker_error(error: &TrackerError) -> u8 {
    match error {
        TrackerError::Retryable { .. } => RETRYABLE,
        TrackerError::Terminal { .. } => TERMINAL,
    }
}
