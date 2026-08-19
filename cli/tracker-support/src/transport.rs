//! The bounds a provider client puts on a remote call, and the one shape
//! adapter between a projection and the port.
//!
//! The base URL is deliberately not held here: it is a transport type, and
//! this crate carries no transport.

use std::time::Duration;

/// Bounds every provider request and every paginated operation runs under.
///
/// The page cap bounds result size rather than time, so the deadline bounds
/// the whole operation separately: twenty pages, multiplied again by Jira's
/// fifty-id chunks, puts a degraded tracker in the tens of minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportConfig {
    pub timeout: Duration,
    pub deadline: Duration,
    pub max_response_bytes: usize,
    pub max_pages: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            deadline: Duration::from_secs(300),
            max_response_bytes: 8 * 1024 * 1024,
            max_pages: 20,
        }
    }
}

/// Adapts a projection to `tracker::RemoteIssue.body`, whose port contract
/// requires exactly one trailing newline where the projection carries none.
#[must_use]
pub fn port_body(projection: &str) -> String {
    let trimmed = projection.trim_end_matches('\n');
    format!("{trimmed}\n")
}
