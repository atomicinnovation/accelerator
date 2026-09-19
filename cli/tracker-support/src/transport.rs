//! The bounds a provider client puts on a remote call, and the one shape
//! adapter between a projection and the port.
//!
//! The base URL is deliberately not held here: it is a transport type, and
//! this crate carries no transport.

use std::time::Duration;

use tracker::Ceiling;
use tracker::DEFAULT_MAX_PAGES;

/// Bounds every provider request and every paginated operation runs under.
///
/// The page caps bound result size rather than time, so the deadline bounds the
/// whole operation separately. Discovery and the keyed reconcile read carry
/// independent caps: broadening discovery must not force the keyed read to
/// cap-abort, and vice versa. A shared paging loop (Linear's `page_all`) is
/// handed the relevant cap by its caller rather than reading one field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportConfig {
    pub timeout: Duration,
    pub deadline: Duration,
    pub max_response_bytes: usize,
    /// The page cap for unkeyed discovery searches.
    pub discovery_max_pages: Ceiling,
    /// The page cap for keyed reconcile reads.
    pub keyed_read_max_pages: Ceiling,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            deadline: Duration::from_secs(300),
            max_response_bytes: 8 * 1024 * 1024,
            discovery_max_pages: DEFAULT_MAX_PAGES,
            keyed_read_max_pages: DEFAULT_MAX_PAGES,
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
