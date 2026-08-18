//! Two classification tables, both per operation, because a single
//! status-to-class table is wrong by construction: the same wire condition can
//! be provable on `create` and unprovable on `update`.

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

/// What the transport observed, in the terms `jira-request.sh:363-442`
/// classifies on.
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

/// The bash exit code the same outcome produces, kept so a diagnostic can name
/// the code a reader will find in the scripts and in `EXIT_CODES.md`.
#[must_use]
pub const fn bash_code(outcome: Outcome) -> u16 {
    match outcome {
        Outcome::Status(400) => 34,
        Outcome::Status(401) => 11,
        Outcome::Status(403) => 12,
        Outcome::Status(404) => 13,
        Outcome::Status(410) => 14,
        Outcome::Status(429) => 19,
        Outcome::NonJsonBody => 16,
        Outcome::Transport => 21,
        Outcome::Status(_) => 20,
    }
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
    build(provably_unapplied, operation, bash_code(outcome), detail)
}

/// The bridge mappers' own tables.
///
/// Transcribed from `_wicr_map_jira` (`work-item-create-remote.sh:105-111`)
/// and `_wiur_map_jira` (`work-item-update-remote.sh:51-57`), and driven by
/// the committed fixture at
/// `cli/tracker-support/tests/fixtures/bridge-exit-code-tables.txt`.
#[must_use]
pub fn classify_bash_code(
    code: u16,
    operation: Operation,
    detail: &str,
) -> TrackerError {
    let shared_retryable =
        matches!(code, 11 | 12 | 13 | 14 | 15 | 17 | 19 | 22 | 34);
    let provably_unapplied = match operation {
        Operation::Create => shared_retryable || (100..=108).contains(&code),
        Operation::Update => shared_retryable || (110..=117).contains(&code),
        Operation::Read => true,
    };
    build(provably_unapplied, operation, code, detail)
}

fn build(
    provably_unapplied: bool,
    operation: Operation,
    code: u16,
    detail: &str,
) -> TrackerError {
    let detail = format!("jira {} ({code}): {detail}", operation.name());
    if provably_unapplied || !operation.mutates() {
        TrackerError::Retryable { detail }
    } else {
        TrackerError::Terminal { detail }
    }
}
