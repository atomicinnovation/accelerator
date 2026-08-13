//! The (dispatcher code x attempt x write-failed) decision table for
//! `create --push`.
//!
//! Port of `work-item-push-decide.sh`, beside `decide` since it is the
//! same kind of pure table over an orchestration outcome, sited in the
//! domain rather than the binary crate where it is hardest to test.
//!
//! Takes the dispatcher code as a bare `u8`, not a named constant:
//! `work_domain_imports_only_permitted` forbids `work` from importing
//! `work_cli`, where the `RETRYABLE`/`TERMINAL`/... constants live. A `u8`
//! parameter is also what keeps the golden's unknown-code row (`99` ->
//! `loud-terminal`) expressible at all — an enum over the four known codes
//! could not represent it.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushOutcome {
    WriteOnce,
    Retry,
    LocalSave,
    LoudTerminal,
}

impl PushOutcome {
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::WriteOnce => "write-once",
            Self::Retry => "retry",
            Self::LocalSave => "local-save",
            Self::LoudTerminal => "loud-terminal",
        }
    }
}

const RETRYABLE: u8 = 70;
const NOT_AVAILABLE: u8 = 72;
const UNRECOGNISED: u8 = 73;

/// Maps a dispatcher outcome to the next action.
///
/// `write_failed` is consulted only when `code == 0` — a local write
/// failure has no dispatcher code of its own, so it is expressed as a flag
/// alongside the success code, exactly as bash's own decision reads it.
#[must_use]
pub const fn push_decide(
    code: u8,
    attempt: u8,
    write_failed: bool,
) -> PushOutcome {
    if code == 0 {
        return if write_failed {
            PushOutcome::LoudTerminal
        } else {
            PushOutcome::WriteOnce
        };
    }

    match code {
        RETRYABLE => {
            if attempt <= 1 {
                PushOutcome::Retry
            } else {
                PushOutcome::LocalSave
            }
        }
        NOT_AVAILABLE | UNRECOGNISED => PushOutcome::LocalSave,
        // Covers the terminal code (71) and every unrecognised code alike:
        // a known terminal failure and an unknown dispatcher code both mean
        // "a remote issue may exist", so both fall back to the
        // conservative default.
        _ => PushOutcome::LoudTerminal,
    }
}
