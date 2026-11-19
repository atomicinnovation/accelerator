//! The templates a `conduct` run's spawns fill, and the one command that
//! fetches each.

use crate::confinement::PermittedCommand;
use crate::confinement::FINDING_TEMPLATE;
use crate::confinement::LEVEL_NOTE_TEMPLATE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConductTemplate {
    Finding,
    LevelNote,
}

impl ConductTemplate {
    pub const SET: &'static str = "topic-research";

    pub const ALL: [Self; 2] = [Self::Finding, Self::LevelNote];

    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Finding => "finding",
            Self::LevelNote => "level-note",
        }
    }

    #[must_use]
    pub fn name(&self) -> String {
        format!("{}-{}", Self::SET, self.kind())
    }

    #[must_use]
    pub const fn permitted_command(self) -> PermittedCommand {
        match self {
            Self::Finding => FINDING_TEMPLATE,
            Self::LevelNote => LEVEL_NOTE_TEMPLATE,
        }
    }
}
