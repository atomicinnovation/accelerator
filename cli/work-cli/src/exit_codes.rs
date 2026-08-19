//! The whole binary's exit-code taxonomy, in one place.

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
pub const UNCONFIGURED: u8 = 74;

#[must_use]
pub const fn for_tracker_error(error: &TrackerError) -> u8 {
    match error {
        TrackerError::Retryable { .. } => RETRYABLE,
        TrackerError::Terminal { .. } => TERMINAL,
    }
}
