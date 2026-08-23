//! The structured failure a fallible port operation surfaces before it
//! collapses to the port's two-class [`TrackerError`].
//!
//! The binary reads the granular bash exit code from this — the classifier's
//! [`Outcome`], whose `bash_code` is the integer, or the distinct post-create
//! "created remotely but unwritable" case — rather than parsing it back out of
//! a `TrackerError` detail string. Every error site of `create`/`update` is
//! funnelled through here, so the port impl derives `TrackerError` from one
//! place and the binary maps the same value straight to an exit code.

use tracker::TrackerError;

use crate::classify::classify;
use crate::classify::Operation;
use crate::classify::Outcome;

/// A create/update failure, carrying the discriminant the binary needs.
#[derive(Debug, Clone)]
pub enum JiraFailure {
    /// A wire outcome the classifier recognises. `bash_code(outcome)` is the
    /// exit code; `operation` decides the retry class the port derives.
    Wire {
        outcome: Outcome,
        operation: Operation,
        detail: String,
    },
    /// A create that succeeded remotely but returned a key that cannot be
    /// written back — the non-retryable "created remotely but unwritable" case
    /// the create flow must distinguish from a pre-send refusal.
    UnwritableIdentifier { identifier: String, reason: String },
}

impl JiraFailure {
    pub(crate) const fn wire(
        outcome: Outcome,
        operation: Operation,
        detail: String,
    ) -> Self {
        Self::Wire {
            outcome,
            operation,
            detail,
        }
    }
}

impl From<JiraFailure> for TrackerError {
    fn from(failure: JiraFailure) -> Self {
        match failure {
            JiraFailure::Wire {
                outcome,
                operation,
                detail,
            } => classify(outcome, operation, &detail),
            JiraFailure::UnwritableIdentifier { identifier, reason } => {
                Self::Terminal {
                    detail: format!(
                        "jira create: the issue was created as {identifier:?}, \
                         which cannot be written back — {reason}"
                    ),
                }
            }
        }
    }
}
