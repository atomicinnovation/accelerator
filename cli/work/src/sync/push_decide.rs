//! The (dispatcher code x attempt x write-failed) decision table for
//! `create --push`.
//!
//! The dispatcher code is a bare `u8` rather than an enum over the codes
//! this crate knows: an unrecognised code must stay expressible, since it is
//! what the conservative default exists to handle.

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
const UNCONFIGURED: u8 = 74;

/// Maps a dispatcher outcome to the next action.
///
/// `write_failed` is a flag rather than a code because a local write
/// failure has no dispatcher code of its own; it is consulted only
/// alongside the success code.
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
        NOT_AVAILABLE | UNRECOGNISED | UNCONFIGURED => PushOutcome::LocalSave,
        // A known terminal failure and an unknown dispatcher code both mean
        // a remote issue may exist, so both take the conservative default.
        _ => PushOutcome::LoudTerminal,
    }
}
