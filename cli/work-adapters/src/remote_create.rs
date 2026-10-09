//! Sending one create to the tracker and classifying what became of it.
//!
//! No marker is written here: each caller keeps the record its own crash
//! recovery depends on around the call.

use tracker::ExternalId;
use tracker::RemoteTracker;
use tracker::TrackerError;

pub struct CreateRequest<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub kind: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteCreate {
    Created(ExternalId),
    /// Provably no issue was created, and retrying did not reach the
    /// tracker.
    TrackerUnreachable,
    /// The request may have been applied: an issue may exist.
    OutcomeUnknown {
        detail: String,
    },
    /// The client refused the request as invalid before sending it.
    Rejected {
        detail: String,
    },
}

const ATTEMPTS: u8 = 2;

/// Sends the create, retrying once only when the first failure proves no
/// issue was created.
#[must_use]
pub fn send_create(
    request: &CreateRequest<'_>,
    tracker: &dyn RemoteTracker,
) -> RemoteCreate {
    let mut attempt = 1;
    loop {
        match tracker.create(request.title, request.body, request.kind) {
            Ok(key) => return RemoteCreate::Created(key),
            Err(TrackerError::Retryable { .. }) if attempt < ATTEMPTS => {
                attempt += 1;
            }
            Err(
                TrackerError::Retryable { .. }
                | TrackerError::Unconfigured { .. },
            ) => return RemoteCreate::TrackerUnreachable,
            Err(TrackerError::Terminal { detail }) => {
                return RemoteCreate::OutcomeUnknown { detail }
            }
            Err(TrackerError::Rejected { detail }) => {
                return RemoteCreate::Rejected { detail }
            }
        }
    }
}
