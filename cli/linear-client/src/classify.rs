//! Linear's classification **parses the response body**, not only the status.
//!
//! Three things a status-only classifier cannot reproduce:
//!
//! - a **200** can carry `errors[]`, and that is a failure
//! - rate limiting arrives as **HTTP 400** with `"code": "RATELIMITED"`
//! - complexity rejection has no machine-readable code, so the only signal is
//!   the word `complexity` in a message
//!
//! Linear emits no 403, 404, 410 or 429 at all — those statuses are Jira-only.

use serde_json::Value;
use tracker::TrackerError;

/// Which port operation a failure belongs to.
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

/// How an `errors[]` array classifies. Order is load-bearing: auth, then
/// complexity, then rate limit, then everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphQlError {
    Auth,
    Complexity,
    RateLimited,
    BadRequest,
}

/// The complexity discriminator, pinned in one place because it is a known-
/// fragile heuristic. The full word is required, not the `complex` stem, so an
/// unrelated "complex query" phrasing does not match.
pub const COMPLEXITY_PATTERN: &str = "complexity";

/// Classifies an `errors[]` array.
#[must_use]
pub fn classify_errors(body: &Value) -> GraphQlError {
    let errors = body
        .get("errors")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, Clone::clone);
    let lowercased = |pointer: &str| -> Vec<String> {
        errors
            .iter()
            .filter_map(|error| error.pointer(pointer))
            .filter_map(Value::as_str)
            .map(str::to_ascii_lowercase)
            .collect()
    };
    let types = lowercased("/extensions/type");
    let codes = lowercased("/extensions/code");
    let messages = errors
        .iter()
        .filter_map(|error| error.get("message"))
        .filter_map(Value::as_str)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();

    let contains = |haystack: &[String], needle: &str| {
        haystack.iter().any(|value| value.contains(needle))
    };

    if contains(&types, "authentication error")
        || contains(&codes, "authentication_error")
    {
        return GraphQlError::Auth;
    }
    if contains(&messages, COMPLEXITY_PATTERN) {
        return GraphQlError::Complexity;
    }
    if contains(&codes, "ratelimited") || contains(&types, "ratelimited") {
        return GraphQlError::RateLimited;
    }
    GraphQlError::BadRequest
}

/// Whether a body carries a non-empty `errors[]` array.
#[must_use]
pub fn carries_errors(body: &Value) -> bool {
    body.get("errors")
        .and_then(Value::as_array)
        .is_some_and(|errors| !errors.is_empty())
}

/// What the transport observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// A 2xx whose body carries `errors[]`, already classified.
    SuccessWithErrors(GraphQlError),
    /// A 2xx whose body is not JSON.
    NonJsonBody,
    /// HTTP 401.
    Unauthorised,
    /// HTTP 400, with its body's classification.
    BadRequest(GraphQlError),
    /// A 5xx whose retries are exhausted.
    ServerError,
    /// A connect, DNS or timeout failure.
    Transport,
    /// Any other status.
    Unexpected,
}

/// Whether a wire outcome proves no mutation happened, for this operation.
///
/// A body error carried by a 2xx or a 400 is retryable on create but terminal
/// on update — a 200-body error may mean the update applied. Auth, complexity,
/// and a 400 rate-limit are provably unapplied on both operations.
#[must_use]
pub fn classify(
    outcome: Outcome,
    operation: Operation,
    detail: &str,
) -> TrackerError {
    let provably_unapplied = match outcome {
        Outcome::SuccessWithErrors(
            GraphQlError::Auth | GraphQlError::Complexity,
        )
        | Outcome::Unauthorised
        | Outcome::BadRequest(
            GraphQlError::Auth
            | GraphQlError::Complexity
            | GraphQlError::RateLimited,
        ) => true,
        Outcome::SuccessWithErrors(
            GraphQlError::RateLimited | GraphQlError::BadRequest,
        )
        | Outcome::BadRequest(GraphQlError::BadRequest) => {
            matches!(operation, Operation::Create)
        }
        Outcome::NonJsonBody
        | Outcome::Transport
        | Outcome::ServerError
        | Outcome::Unexpected => false,
    };
    let detail = format!("linear {}: {detail}", operation.name());
    if provably_unapplied || !operation.mutates() {
        TrackerError::Retryable { detail }
    } else {
        TrackerError::Terminal { detail }
    }
}
