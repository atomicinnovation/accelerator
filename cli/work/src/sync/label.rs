//! The status glyph table `/list-work-items` renders.

use crate::sync::state::SyncState;

/// The five states that carry a rendered label. `RemoteAbsent` and
/// `Indeterminate` have none, so a caller cannot ask for a glyph that does
/// not exist.
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

/// Reports whether anything survives stripping whitespace and quotes as one
/// *combined* class from both ends.
///
/// Stripping quotes and then trimming is a different function: it
/// classifies `  'PROJ-1'  ` differently, since the combined class removes
/// both in one pass regardless of their order.
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

/// The rendered `<glyph> <text>` label, with no trailing newline.
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
