//! The error every provider-surface operation returns.
//!
//! These operations sit beyond the four `RemoteTracker` port methods — comment,
//! transition, attach, discovery — and have no port of their own yet. Until
//! one exists each returns this concrete error, whose variants keep the
//! distinctions a caller needs (an unknown state from an ambiguous one, a
//! refused upload host from a failed PUT) rather than flattening them to a
//! string.

use serde_json::Value;
use thiserror::Error;

use crate::error::ClientError;

#[derive(Debug, Error)]
pub enum SurfaceError {
    #[error(transparent)]
    Client(#[from] ClientError),

    #[error("E_GQL_ERRORS: {operation} returned errors: {body}")]
    GraphQlErrors {
        operation: &'static str,
        body: String,
    },

    #[error("E_REQ_STATUS: {operation} returned HTTP {status}")]
    Status {
        operation: &'static str,
        status: u16,
        body: String,
    },

    #[error("E_REQ_BAD_RESPONSE: {operation}: {reason}")]
    BadResponse {
        operation: &'static str,
        reason: String,
    },

    #[error(
        "E_TRANSITION_STATE_NOT_IN_CATALOGUE: state {name:?} is not in the \
         catalogue — run /init-linear to refresh it"
    )]
    UnknownState { name: String },

    #[error(
        "E_TRANSITION_STATE_AMBIGUOUS: state name {name:?} matches {count} \
         catalogue states and cannot be disambiguated"
    )]
    AmbiguousState { name: String, count: usize },

    #[error("E_ATTACH_BAD_URL: {url:?} must be an http(s) URL")]
    BadLinkUrl { url: String },

    #[error("E_ATTACH_FILE_MISSING: {path}: {reason}")]
    FileRefused { path: String, reason: String },

    #[error(
        "E_ATTACH_BAD_UPLOAD_URL: refusing to trust a server-supplied \
         {role} host {host:?} outside *.linear.app"
    )]
    BadUploadUrl { role: &'static str, host: String },

    #[error(
        "E_ATTACH_BAD_HEADER: the server echoed header {name:?} carries a CR \
         or LF and is refused"
    )]
    BadEchoedHeader { name: String },

    #[error(
        "E_ATTACH_UPLOAD_FAILED: the PUT to {url} failed after {attempts} \
         attempts ({detail})"
    )]
    UploadFailed {
        url: String,
        attempts: usize,
        detail: String,
    },

    #[error(
        "E_ATTACH_REGISTER_FAILED: the file was uploaded to {asset_url} but \
         attachmentCreate failed ({detail}); the asset is orphaned in Linear \
         and a re-run re-uploads"
    )]
    RegisterFailed { asset_url: String, detail: String },
}

impl SurfaceError {
    /// Builds a [`SurfaceError::Status`] from a non-2xx response.
    pub(crate) const fn status(
        operation: &'static str,
        status: u16,
        body: String,
    ) -> Self {
        Self::Status {
            operation,
            status,
            body,
        }
    }
}

/// Interprets a GraphQL response the way the surface flows do: a non-JSON body
/// or a body carrying `errors[]` is a failure, whatever the status; otherwise
/// the parsed body is returned for the caller to project.
pub(crate) fn interpret(
    received: &crate::transport::Received,
    operation: &'static str,
) -> Result<Value, SurfaceError> {
    let Some(body) = received.json() else {
        if (200..300).contains(&received.status) {
            return Err(SurfaceError::BadResponse {
                operation,
                reason: "the response was not JSON".to_owned(),
            });
        }
        return Err(SurfaceError::status(
            operation,
            received.status,
            received.body.clone(),
        ));
    };
    if crate::classify::carries_errors(&body) {
        return Err(SurfaceError::GraphQlErrors {
            operation,
            body: received.body.clone(),
        });
    }
    if !(200..300).contains(&received.status) {
        return Err(SurfaceError::status(
            operation,
            received.status,
            received.body.clone(),
        ));
    }
    Ok(body)
}
