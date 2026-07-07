//! The two configuration levels.

use std::fmt::{self, Display, Formatter};

/// A configuration level. Personal wins over team when both resolve a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Team,
    Personal,
}

impl Display for Level {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Team => write!(formatter, "team"),
            Self::Personal => write!(formatter, "personal"),
        }
    }
}
