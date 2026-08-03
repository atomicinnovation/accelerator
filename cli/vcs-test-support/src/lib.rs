//! The shared VCS test apparatus: the checkout fixture matrix, the hermetic
//! environment it is built in, and (from the zero-spawn work) the marker
//! stubs.
//!
//! Published as a crate rather than a feature on `vcs-adapters` for two
//! reasons: it keeps that crate's `[features]` at exactly `bash-parity`, and it
//! sidesteps CI's `--all-features` turning a fixture feature on workspace-wide.
//! It is consumed from more than one crate, so the shadow list has one
//! definition.

pub mod fixtures;
pub mod hermetic;
pub mod stubs;

use std::fmt;

/// Anything that can go wrong building or describing a fixture.
///
/// A single opaque error on purpose: callers are test harnesses that report and
/// abort, never branch on the cause.
#[derive(Debug)]
pub struct Error(String);

impl Error {
    #[must_use]
    pub fn message(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}
