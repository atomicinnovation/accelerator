//! Per-operation retry classification.
//!
//! A single status-to-class table is wrong by construction: the same wire
//! condition can be provable on `create` and unprovable on `update`, so the
//! retry class is a function of the outcome and the operation together, never
//! the status alone.

use tracker::TrackerError;

/// Which port operation a failure belongs to. A read never produces
/// `Terminal` — there was nothing to mutate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Create,
    Update,
    Read,
}

impl Operation {
    const fn mutates(self) -> bool {
        matches!(self, Self::Create | Self::Update)
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Read => "read",
        }
    }
}

/// What the transport observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A response arrived with this status; 429 and 5xx here mean the retries
    /// were already exhausted.
    Status(u16),
    /// A 2xx whose body is not JSON.
    NonJsonBody,
    /// A connect, DNS or timeout failure.
    Transport,
}

/// Whether a wire outcome proves no mutation happened, for this operation.
#[must_use]
pub fn classify(
    outcome: Outcome,
    operation: Operation,
    detail: &str,
) -> TrackerError {
    let provably_unapplied = match outcome {
        Outcome::Status(400 | 401 | 403 | 404 | 410 | 429) => true,
        Outcome::Status(_) | Outcome::NonJsonBody | Outcome::Transport => false,
    };
    let detail = format!("jira {}: {detail}", operation.name());
    if provably_unapplied || !operation.mutates() {
        TrackerError::Retryable { detail }
    } else {
        TrackerError::Terminal { detail }
    }
}
