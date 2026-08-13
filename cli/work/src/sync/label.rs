//! The status glyph table `/list-work-items` renders. Port of
//! `work-item-sync-label.sh`.

use crate::sync::state::SyncState;

/// The five states that carry a rendered label.
///
/// `RemoteAbsent` and `Indeterminate` have none — the script's `--label` arm
/// rejects them with exit 1 — so a caller cannot ask this module for a
/// glyph that does not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderableState {
    Synced,
    Unsynced,
    LocallyModified,
    RemotelyModified,
    Conflict,
}

impl TryFrom<SyncState> for RenderableState {
    type Error = ();

    fn try_from(state: SyncState) -> Result<Self, Self::Error> {
        Ok(match state {
            SyncState::Synced => Self::Synced,
            SyncState::Unsynced => Self::Unsynced,
            SyncState::LocallyModified => Self::LocallyModified,
            SyncState::RemotelyModified => Self::RemotelyModified,
            SyncState::Conflict => Self::Conflict,
            SyncState::RemoteAbsent | SyncState::Indeterminate => {
                return Err(())
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPresence {
    Synced,
    Unsynced,
}

/// Strips bash's *combined* `[[:space:]"']` character class from both ends —
/// not quotes, then whitespace — and reports whether anything survives.
///
/// A "strip quotes, then trim" port is a different function: it classifies
/// `  'PROJ-1'  ` differently, since the combined class removes the
/// surrounding quotes and whitespace in one pass regardless of which comes
/// first or last.
#[must_use]
pub fn classify_external_id(raw: &str) -> SyncPresence {
    let is_strippable =
        |c: char| c.is_ascii_whitespace() || c == '"' || c == '\'';
    let trimmed = raw.trim_matches(is_strippable);
    if trimmed.is_empty() {
        SyncPresence::Unsynced
    } else {
        SyncPresence::Synced
    }
}

/// The rendered `<glyph> <text>` label, with no trailing newline — matching
/// `printf` without `\n` (`work-item-sync-label.sh:57-61`).
#[must_use]
pub const fn label(state: RenderableState) -> &'static str {
    match state {
        RenderableState::Synced => "🟢 synced",
        RenderableState::Unsynced => "⚪ unsynced",
        RenderableState::LocallyModified => "🔵 locally modified",
        RenderableState::RemotelyModified => "🟣 remotely modified",
        RenderableState::Conflict => "🔴 conflict",
    }
}
