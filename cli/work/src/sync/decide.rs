//! The (direction × state × dirty) decision table.

use crate::sync::state::SyncState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Bidirectional,
    PushOnly,
    PullOnly,
}

/// Three-valued, so a failed probe and a clean tree are never one value.
///
/// `Unknown` decides as `Dirty` everywhere, since VCS revert cannot recover
/// the uncommitted working-copy changes an overwrite would destroy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dirtiness {
    Clean,
    Dirty,
    Unknown,
}

/// One reconciliation action for a single work item.
///
/// The `CreateFromRemote` and `CreateFromLocal` variants are never returned by
/// [`decide`]: the decision table maps `Unsynced` to `Noop` in every direction,
/// as its frozen bash oracle records, because remote-issue creation was always
/// separate orchestration rather than a table entry. The sync engine produces
/// the two create actions out-of-band and reports them under these keywords;
/// they exist here so a report round-trips and the engine's exhaustive action
/// match names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Push,
    Pull,
    SkipConflict,
    SkipDirty,
    Prompt,
    Noop,
    CreateFromRemote,
    CreateFromLocal,
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
            "create-from-remote" => Self::CreateFromRemote,
            "create-from-local" => Self::CreateFromLocal,
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
            Self::Prompt => "unresolved",
            Self::Noop => "noop",
            Self::CreateFromRemote => "create-from-remote",
            Self::CreateFromLocal => "create-from-local",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    AcceptRemote,
    PushLocal,
    Skip,
}

/// Maps (direction, state, dirty) to one action. `dirty` sub-splits
/// `RemotelyModified` alone; every other state ignores it.
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

/// Folds case then trims, in ASCII only.
///
/// A Unicode-aware `to_lowercase()`/`trim()` would resolve a leading U+00A0
/// to `AcceptRemote`, turning the deliberately safe default into a local
/// overwrite for whitespace no human can see.
///
/// `None` for an unrecognised token, so a caller can warn about it before
/// applying [`Resolution::Skip`] itself.
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

#[cfg(test)]
mod tests {
    use super::Action;

    #[test]
    fn every_action_round_trips_through_its_keyword() {
        let variants = [
            Action::Push,
            Action::Pull,
            Action::SkipConflict,
            Action::SkipDirty,
            Action::Prompt,
            Action::Noop,
            Action::CreateFromRemote,
            Action::CreateFromLocal,
        ];
        for variant in variants {
            let keyword = variant.to_string();
            assert_eq!(Action::from_keyword(&keyword), Some(variant));
        }
    }

    #[test]
    fn the_two_create_actions_never_come_from_the_decision_table() {
        use super::decide;
        use super::Dirtiness;
        use super::SyncDirection;
        use crate::sync::state::SyncState;

        let states = [
            SyncState::Synced,
            SyncState::Unsynced,
            SyncState::LocallyModified,
            SyncState::RemotelyModified,
            SyncState::Conflict,
            SyncState::RemoteAbsent,
            SyncState::Indeterminate,
        ];
        let directions = [
            SyncDirection::Bidirectional,
            SyncDirection::PushOnly,
            SyncDirection::PullOnly,
        ];
        let dirtiness =
            [Dirtiness::Clean, Dirtiness::Dirty, Dirtiness::Unknown];
        for state in states {
            for direction in directions {
                for dirty in dirtiness {
                    let action = decide(direction, state, dirty);
                    assert!(
                        !matches!(
                            action,
                            Action::CreateFromRemote | Action::CreateFromLocal
                        ),
                        "decide produced a create action for \
                         {direction:?}/{state:?}/{dirty:?}"
                    );
                }
            }
        }
    }
}
