//! The seven-keyword sync classification vocabulary.

use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Synced,
    Unsynced,
    LocallyModified,
    RemotelyModified,
    Conflict,
    RemoteAbsent,
    Indeterminate,
}

impl SyncState {
    /// The inverse of [`Display`]. `None` for an unrecognised keyword.
    #[must_use]
    pub fn from_keyword(raw: &str) -> Option<Self> {
        Some(match raw {
            "synced" => Self::Synced,
            "unsynced" => Self::Unsynced,
            "locally-modified" => Self::LocallyModified,
            "remotely-modified" => Self::RemotelyModified,
            "conflict" => Self::Conflict,
            "remote-absent" => Self::RemoteAbsent,
            "indeterminate" => Self::Indeterminate,
            _ => return None,
        })
    }
}

impl Display for SyncState {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Synced => "synced",
            Self::Unsynced => "unsynced",
            Self::LocallyModified => "locally-modified",
            Self::RemotelyModified => "remotely-modified",
            Self::Conflict => "conflict",
            Self::RemoteAbsent => "remote-absent",
            Self::Indeterminate => "indeterminate",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SyncState;

    #[test]
    fn every_variant_round_trips_through_its_keyword() {
        let variants = [
            SyncState::Synced,
            SyncState::Unsynced,
            SyncState::LocallyModified,
            SyncState::RemotelyModified,
            SyncState::Conflict,
            SyncState::RemoteAbsent,
            SyncState::Indeterminate,
        ];
        for variant in variants {
            let keyword = variant.to_string();
            assert_eq!(SyncState::from_keyword(&keyword), Some(variant));
        }
    }

    #[test]
    fn an_unrecognised_keyword_parses_to_none() {
        assert_eq!(SyncState::from_keyword("bogus"), None);
    }
}
