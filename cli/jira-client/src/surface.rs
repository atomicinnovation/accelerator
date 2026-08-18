//! The error every provider-surface operation returns.
//!
//! These operations sit beyond the four `RemoteTracker` port methods — comment,
//! transition, attach, discovery — and have no port of their own yet: the right
//! shape is only visible once a caller exists. Until then each returns
//! this concrete error, whose variants keep the distinctions a caller needs
//! (a not-found transition from an ambiguous one, a refused filename from a
//! failed request) rather than flattening them to a string.

use thiserror::Error;

use crate::adf::AdfError;
use crate::error::ClientError;
use crate::transition::Transition;

#[derive(Debug, Error)]
pub enum SurfaceError {
    #[error(transparent)]
    Client(#[from] ClientError),

    #[error(transparent)]
    Adf(#[from] AdfError),

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
        "E_COMMENT_BAD_PAGE_SIZE: --page-size must be in [1, 100], got {got}"
    )]
    BadPageSize { got: u32 },

    #[error(
        "E_TRANSITION_NOT_FOUND: no transition leads to state {state:?} from \
         the current state"
    )]
    TransitionNotFound { state: String },

    #[error(
        "E_TRANSITION_AMBIGUOUS: {} transitions lead to state {state:?}",
        matches.len()
    )]
    TransitionAmbiguous {
        state: String,
        matches: Vec<Transition>,
    },

    #[error(
        "E_ATTACH_BAD_FILENAME: {name:?} cannot appear in a multipart \
         filename — it carries a quote, newline or control byte"
    )]
    BadFilename { name: String },

    #[error("E_ATTACH_FILE_MISSING: {path}: {reason}")]
    FileRefused { path: String, reason: String },

    #[error(
        "E_ATTACH_ESCAPE: {path} resolves outside the repository root and \
         cannot be uploaded"
    )]
    PathEscapesRoot { path: String },

    #[error(
        "E_ATTACH_BOUNDARY: no multipart boundary free of every part body \
         could be generated"
    )]
    BoundaryExhausted,
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
