//! The shared VCS test apparatus: the checkout fixture matrix, the hermetic
//! environment it is built in, and the marker stubs.
//!
//! A crate rather than a feature on `vcs-adapters`, so that crate's `[features]`
//! stays at exactly `bash-parity` and CI's `--all-features` cannot turn a fixture
//! feature on workspace-wide.

pub mod fixtures;
pub mod hermetic;
pub mod masks;
pub mod stubs;

use std::fmt;

/// A single opaque error: callers are test harnesses that report and abort,
/// never branch on the cause.
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
