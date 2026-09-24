//! The run base: the commits a migration run's working copy was based on
//! when it began, which a later run compares against to decide whether the
//! run's own output can be resumed over.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunBase(String);

impl RunBase {
    /// The run base for a working copy based on `ids`, joined in sorted
    /// order so a merge's parent order does not matter. `None` for a working
    /// copy based on nothing, such as an unborn git `HEAD`.
    #[must_use]
    pub fn from_base_commits(ids: &[String]) -> Option<Self> {
        if ids.is_empty() {
            return None;
        }
        let mut sorted = ids.to_vec();
        sorted.sort_unstable();
        Some(Self(sorted.join("+")))
    }

    /// Reads back whatever an earlier run recorded: a record in any other
    /// encoding still compares, and never matches.
    #[must_use]
    pub fn recorded(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_owned()))
    }
}

impl fmt::Display for RunBase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
