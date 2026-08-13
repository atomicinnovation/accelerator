//! The (direction × state × dirty) decision table. Port of
//! `work-item-sync-decide.sh`'s `decide` and `resolve-conflict-token`
//! subcommands.

use crate::sync::state::SyncState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Bidirectional,
    PushOnly,
    PullOnly,
}

/// Three-valued so absence and cleanliness are never the same value.
///
/// `work::file_dirty` already maps a failed VCS status probe to dirty
/// deliberately — the recovery model is VCS revert, which cannot recover
/// uncommitted working-copy changes — and `Unknown` preserves that: it
/// decides as `Dirty` everywhere, so a failed probe can never authorise an
/// overwrite by defaulting quietly to clean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dirtiness {
    Clean,
    Dirty,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Push,
    Pull,
    SkipConflict,
    SkipDirty,
    Prompt,
    Noop,
}

impl Action {
    #[must_use]
    pub fn from_keyword(raw: &str) -> Option<Self> {
        Some(match raw {
            "push" => Self::Push,
            "pull" => Self::Pull,
            "skip-conflict" => Self::SkipConflict,
            "skip-dirty" => Self::SkipDirty,
            "unresolved" => Self::Prompt,
            "noop" => Self::Noop,
            _ => return None,
        })
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Push => "push",
            Self::Pull => "pull",
            Self::SkipConflict => "skip-conflict",
            Self::SkipDirty => "skip-dirty",
            // The report's one deliberate divergence from the bash keyword
            // (`prompt`): it carries the exit-code semantics, so owning the
            // wire spelling here keeps one place holding it rather than a
            // second hand-rolled mapping in the CLI layer.
            Self::Prompt => "unresolved",
            Self::Noop => "noop",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    AcceptRemote,
    PushLocal,
    Skip,
}

/// Maps (direction, state, dirty) to one action.
///
/// The table is 7 states x 3 directions, with `dirty` sub-splitting
/// `RemotelyModified` only — every other state collapses to the same action
/// regardless of direction or dirtiness where the direction forbids the
/// write, or to `Noop` where there is nothing to reconcile.
#[must_use]
pub const fn decide(
    direction: SyncDirection,
    state: SyncState,
    dirty: Dirtiness,
) -> Action {
    match state {
        SyncState::Synced
        | SyncState::RemoteAbsent
        | SyncState::Indeterminate
        | SyncState::Unsynced => Action::Noop,
        SyncState::LocallyModified => match direction {
            SyncDirection::Bidirectional | SyncDirection::PushOnly => {
                Action::Push
            }
            SyncDirection::PullOnly => Action::Noop,
        },
        SyncState::RemotelyModified => match direction {
            SyncDirection::PushOnly => Action::Noop,
            SyncDirection::Bidirectional => {
                if matches!(dirty, Dirtiness::Clean) {
                    Action::Pull
                } else {
                    Action::Prompt
                }
            }
            SyncDirection::PullOnly => {
                if matches!(dirty, Dirtiness::Clean) {
                    Action::Pull
                } else {
                    Action::SkipDirty
                }
            }
        },
        SyncState::Conflict => match direction {
            SyncDirection::Bidirectional => Action::Prompt,
            SyncDirection::PushOnly | SyncDirection::PullOnly => {
                Action::SkipConflict
            }
        },
    }
}

/// Folds case then trims in ASCII only, matching bash's `tr '[:upper:]'
/// '[:lower:]'` plus a `[[:space:]]` sed in the C locale.
///
/// Unicode-aware `to_lowercase()`/`trim()` would resolve a leading U+00A0 to
/// `AcceptRemote` where bash leaves it unrecognised and skips — turning the
/// deliberately safe default into a local overwrite for whitespace no human
/// can see.
///
/// `None` when the token is not one this resolver recognises; the caller
/// maps that to [`Resolution::Skip`] itself, so the safe default is
/// unchanged while still letting a caller warn that a token went
/// unrecognised.
#[must_use]
pub fn resolve_conflict_token(raw: &str) -> Option<Resolution> {
    let folded = raw.to_ascii_lowercase();
    let trimmed = folded.trim_matches(|c: char| c.is_ascii_whitespace());
    match trimmed {
        "remote" => Some(Resolution::AcceptRemote),
        "local" => Some(Resolution::PushLocal),
        "skip" => Some(Resolution::Skip),
        _ => None,
    }
}
