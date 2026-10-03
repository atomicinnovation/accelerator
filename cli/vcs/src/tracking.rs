//! Whether a file is tracked, and which repositories enclose a directory, as
//! every repository enclosing it answers.

use std::path::PathBuf;

/// A file's tracking status across every enclosing repository. `Unknown`
/// means a repository was detected but could not answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTracking {
    Untracked,
    Tracked,
    Unknown,
}

impl FileTracking {
    /// Folds every repository's answer: `Tracked` beats `Unknown` beats
    /// `Untracked`, so no enclosing repository answers means `Untracked`, and
    /// a repository that tracks the file wins over one that failed.
    #[must_use]
    pub fn combine(answers: impl IntoIterator<Item = Self>) -> Self {
        answers.into_iter().fold(Self::Untracked, |folded, answer| {
            match (folded, answer) {
                (Self::Tracked, _) | (_, Self::Tracked) => Self::Tracked,
                (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
                (Self::Untracked, Self::Untracked) => Self::Untracked,
            }
        })
    }
}

/// The roots of every repository enclosing a directory. `complete` is false
/// when a repository was detected but one of its roots could not be
/// determined, so the set may be missing a root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootsAnswer {
    pub roots: Vec<PathBuf>,
    pub complete: bool,
}

#[cfg(test)]
mod tests {
    use super::FileTracking;
    use super::FileTracking::{Tracked, Unknown, Untracked};

    #[test]
    fn no_enclosing_repository_is_untracked() {
        assert_eq!(FileTracking::combine([]), Untracked);
    }

    #[test]
    fn tracked_beats_unknown_beats_untracked_in_any_order() {
        let all = [Untracked, Tracked, Unknown];
        for first in all {
            for second in all {
                let expected = if first == Tracked || second == Tracked {
                    Tracked
                } else if first == Unknown || second == Unknown {
                    Unknown
                } else {
                    Untracked
                };
                assert_eq!(
                    FileTracking::combine([first, second]),
                    expected,
                    "{first:?} then {second:?}"
                );
            }
            assert_eq!(FileTracking::combine([first]), first);
        }
        assert_eq!(
            FileTracking::combine([Unknown, Untracked, Tracked]),
            Tracked
        );
    }
}
