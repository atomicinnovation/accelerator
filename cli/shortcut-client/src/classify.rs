//! Per-operation retry classification, the same rule as `jira-client`'s.
//!
//! A failed write is `Retryable` only when it provably applied nothing: it was
//! never sent, or Shortcut answered with a status it uses to reject a request
//! before applying it. The provability covers every attempt, not the last
//! alone — an earlier 5xx or lost answer may have applied the write.

use serde_json::Value;
use tracker::TrackerError;

use crate::transport::Received;

const TOKEN_PAGE: &str = "https://app.shortcut.com/settings/account/api-tokens";

/// Which call a failure belongs to. A read never produces `Terminal` — there
/// was nothing to mutate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Create,
    Update,
    Comment,
    Transition,
    Attach,
    Read,
}

impl Operation {
    const fn mutates(self) -> bool {
        !matches!(self, Self::Read)
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Comment => "comment",
            Self::Transition => "transition",
            Self::Attach => "attach",
            Self::Read => "read",
        }
    }
}

/// What the transport, or the operation decoding its answer, observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A response that is not 2xx, whatever its body; 429 and 5xx here mean
    /// the retries were already exhausted.
    Status(u16),
    /// A 2xx whose body is not JSON.
    NonJsonBody,
    /// A 2xx JSON body without a field the operation needs.
    UnusableBody,
    /// A timeout, or a connection lost after the request left the client.
    Transport,
    /// A connect or DNS failure before any byte was sent.
    NotSent,
    /// A request the client refused as invalid before sending it.
    RequestInvalid,
}

impl Outcome {
    const fn provably_unapplied(self) -> bool {
        matches!(
            self,
            Self::NotSent
                | Self::RequestInvalid
                | Self::Status(400 | 401 | 403 | 404 | 409 | 410 | 422 | 429)
        )
    }
}

/// One failed request: its last outcome, and whether any attempt may have
/// applied it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub outcome: Outcome,
    pub may_have_applied: bool,
}

#[must_use]
pub fn classify(
    failure: Failure,
    operation: Operation,
    detail: &str,
) -> TrackerError {
    let detail = format!("shortcut {}: {detail}", operation.name());
    if !operation.mutates() {
        TrackerError::Retryable { detail }
    } else if failure.may_have_applied {
        TrackerError::Terminal { detail }
    } else if failure.outcome == Outcome::RequestInvalid {
        TrackerError::Rejected { detail }
    } else if failure.outcome.provably_unapplied() {
        TrackerError::Retryable { detail }
    } else {
        TrackerError::Terminal { detail }
    }
}

/// A sentence for a non-2xx answer: what Shortcut said, and what the user can
/// do about it. `scopes` are the token's scopes as whoami reported them.
#[must_use]
pub fn explain(received: &Received, scopes: &[String]) -> String {
    let said = envelope(received);
    match received.status {
        401 => format!(
            "Shortcut refused the token (401){said}. Check it is current: set \
             ACCELERATOR_SHORTCUT_TOKEN or ACCELERATOR_SHORTCUT_TOKEN_CMD, or \
             shortcut.token or shortcut.token_cmd in \
             .accelerator/config.local.md. Create one at {TOKEN_PAGE}"
        ),
        403 if is_read_only(scopes) => format!(
            "Shortcut refused the request (403){said}: the token has only the \
             read scope, and a read-only token cannot write. Create a \
             read-write token at {TOKEN_PAGE}"
        ),
        403 => format!(
            "Shortcut refused the request (403){said}: the token lacks the \
             scope this request needs"
        ),
        status => format!("Shortcut answered {status}{said}"),
    }
}

fn is_read_only(scopes: &[String]) -> bool {
    !scopes.is_empty() && scopes.iter().all(|scope| scope == "read")
}

fn envelope(received: &Received) -> String {
    let Some(body) = received.json() else {
        return String::new();
    };
    let text = |field: &str| {
        body.get(field)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
    };
    match (text("message"), text("tag")) {
        (Some(message), Some(tag)) => format!(" — {message} [{tag}]"),
        (Some(message), None) => format!(" — {message}"),
        (None, Some(tag)) => format!(" [{tag}]"),
        (None, None) => String::new(),
    }
}
