//! The two configuration levels.

use std::fmt::Display;
use std::fmt::Formatter;

/// A configuration level. Personal wins over team when both resolve a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Team,
    Personal,
}

impl Level {
    /// The project-relative file each level is read from.
    #[must_use]
    pub const fn filename(self) -> &'static str {
        match self {
            Self::Team => ".accelerator/config.md",
            Self::Personal => ".accelerator/config.local.md",
        }
    }
}

impl Display for Level {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Team => write!(formatter, "team"),
            Self::Personal => write!(formatter, "personal"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Level;

    #[test]
    fn filename_maps_each_level_to_its_project_relative_file() {
        assert_eq!(Level::Team.filename(), ".accelerator/config.md");
        assert_eq!(Level::Personal.filename(), ".accelerator/config.local.md");
    }
}
