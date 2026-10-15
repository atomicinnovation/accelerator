//! The (direction × state × dirty) decision table.

pub use crate::dirtiness::Dirtiness;
use crate::sync::classify::Classification;
use crate::sync::classify::FieldDelivery;
use crate::sync::classify::LocalReconciliation;
use crate::sync::classify::SyncedReconciliation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Bidirectional,
    PushOnly,
    PullOnly,
}

/// One reconciliation action for a single work item.
///
/// The `CreateFromRemote` and `CreateFromLocal` variants are never returned by
/// [`decide`]: the decision table maps `Unsynced` to `Noop` in every direction,
/// because remote-issue creation was always separate orchestration rather than a
/// table entry. The sync engine produces the two create actions out-of-band and
/// reports them under these keywords; they exist here so a report round-trips and
/// the engine's exhaustive action match names them.
///
/// `Overwrite` is never returned by [`decide`] either: it is a `Prompt`
/// resolved in the local side's favour, which replaces the remote whole,
/// title included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Push(FieldDelivery),
    /// Sends the local title alone, leaving the remote description as it is.
    Retitle,
    Overwrite,
    Pull,
    SkipConflict,
    SkipDirty,
    Prompt,
    Noop,
    RecordBaseline,
    CreateFromRemote,
    CreateFromLocal,
}

impl Action {
    #[must_use]
    pub const fn sends_to_tracker(self) -> bool {
        matches!(self, Self::Push(_) | Self::Retitle | Self::Overwrite)
    }

    /// The action a keyword names; `push` reads as a push that sends the
    /// title, since the keyword does not say whether it is withheld.
    #[must_use]
    pub fn from_keyword(raw: &str) -> Option<Self> {
        Some(match raw {
            "push" => Self::Push(FieldDelivery::Send),
            "retitle" => Self::Retitle,
            "overwrite" => Self::Overwrite,
            "pull" => Self::Pull,
            "skip-conflict" => Self::SkipConflict,
            "skip-dirty" => Self::SkipDirty,
            "unresolved" => Self::Prompt,
            "noop" => Self::Noop,
            "record-baseline" => Self::RecordBaseline,
            "create-from-remote" => Self::CreateFromRemote,
            "create-from-local" => Self::CreateFromLocal,
            _ => return None,
        })
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Push(_) => "push",
            Self::Retitle => "retitle",
            Self::Overwrite => "overwrite",
            Self::Pull => "pull",
            Self::SkipConflict => "skip-conflict",
            Self::SkipDirty => "skip-dirty",
            Self::Prompt => "unresolved",
            Self::Noop => "noop",
            Self::RecordBaseline => "record-baseline",
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

/// Maps (direction, classification, dirty) to one action.
///
/// `dirty` sub-splits `RemotelyModified` alone; every other classification
/// ignores it. An item whose baseline only needs rewriting is recorded in
/// every direction, since recording writes neither side.
#[must_use]
pub const fn decide(
    direction: SyncDirection,
    classification: &Classification,
    dirty: Dirtiness,
) -> Action {
    match classification {
        Classification::Synced(SyncedReconciliation::RecordOnly) => {
            Action::RecordBaseline
        }
        Classification::Synced(SyncedReconciliation::ByContent)
        | Classification::RemoteAbsent
        | Classification::Indeterminate
        | Classification::Unsynced => Action::Noop,
        Classification::LocallyModified(reconciliation) => {
            match (reconciliation, direction) {
                (LocalReconciliation::RecordOnly, _) => Action::RecordBaseline,
                (_, SyncDirection::PullOnly) => Action::Noop,
                (LocalReconciliation::ByContent { fields }, _) => {
                    Action::Push(*fields)
                }
                (LocalReconciliation::SendFields, _) => Action::Retitle,
            }
        }
        Classification::RemotelyModified => match direction {
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
        Classification::Conflict => match direction {
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
            Action::Push(super::FieldDelivery::Send),
            Action::Retitle,
            Action::Overwrite,
            Action::Pull,
            Action::SkipConflict,
            Action::SkipDirty,
            Action::Prompt,
            Action::Noop,
            Action::RecordBaseline,
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
        use super::Classification;
        use super::Dirtiness;
        use super::FieldDelivery;
        use super::LocalReconciliation;
        use super::SyncDirection;
        use super::SyncedReconciliation;

        let states = [
            Classification::Synced(SyncedReconciliation::ByContent),
            Classification::Synced(SyncedReconciliation::RecordOnly),
            Classification::Unsynced,
            Classification::LocallyModified(LocalReconciliation::ByContent {
                fields: FieldDelivery::Send,
            }),
            Classification::LocallyModified(LocalReconciliation::ByContent {
                fields: FieldDelivery::Withhold,
            }),
            Classification::LocallyModified(LocalReconciliation::RecordOnly),
            Classification::LocallyModified(LocalReconciliation::SendFields),
            Classification::RemotelyModified,
            Classification::Conflict,
            Classification::RemoteAbsent,
            Classification::Indeterminate,
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
                    let action = decide(direction, &state, dirty);
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
