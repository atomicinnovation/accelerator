//! The `vcs::RepositoryProbe`-backed implementation of [`WorkingCopyStatus`].
//!
//! One whole-tree diff, taken once at construction and answered from memory
//! thereafter, so a corpus-sized run probes the repository exactly once. The
//! repository is located and classified by `vcs` itself, so a colocated
//! checkout is read through jj and a jj secondary workspace roots at its own
//! working copy.
//!
//! Every unanswerable case — no repository above the start path, a diff the
//! pinned libraries could not compute, a path outside the working copy —
//! folds to [`Dirtiness::Unknown`] rather than a guess.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use tracing::warn;
use vcs::RepositoryProbe;
use work::sync::Dirtiness;

use crate::sync::fetch::WorkingCopyStatus;

pub struct VcsWorkingCopyStatus {
    root: PathBuf,
    dirty: Option<BTreeSet<String>>,
}

impl VcsWorkingCopyStatus {
    /// Diffs the working copy containing `start`.
    #[must_use]
    pub fn probed_from(start: &Path, repository: &dyn RepositoryProbe) -> Self {
        let Some(facts) = repository.facts_at(start) else {
            return Self {
                root: start.to_path_buf(),
                dirty: None,
            };
        };
        let dirty = repository
            .working_copy_state(&facts.root, facts.kind)
            .map(|state| state.dirty_paths.into_iter().collect())
            .map_err(|error| {
                warn!(
                    root = %facts.root.display(),
                    %error,
                    "could not compute the working-copy diff; every item's \
                     dirtiness is unknown"
                );
            })
            .ok();
        Self {
            root: facts.root,
            dirty,
        }
    }
}

impl WorkingCopyStatus for VcsWorkingCopyStatus {
    fn is_dirty(&self, path: &Path) -> Dirtiness {
        let Some(dirty) = &self.dirty else {
            return Dirtiness::Unknown;
        };
        let Some(relative) = repo_relative(&self.root, path) else {
            return Dirtiness::Unknown;
        };
        if dirty.contains(&relative) {
            Dirtiness::Dirty
        } else {
            Dirtiness::Clean
        }
    }
}

/// The probe reports canonicalised roots and forward-slash-separated
/// repo-relative paths; the caller's path arrives however the corpus walk
/// built it, so both sides are brought to that shape before comparison.
fn repo_relative(root: &Path, path: &Path) -> Option<String> {
    let canonical = path.canonicalize();
    let path = canonical.as_deref().unwrap_or(path);
    let relative = path.strip_prefix(root).ok()?;
    Some(
        relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use vcs::{
        RepoFacts, RepositoryProbe, VcsKind, WorkingCopyState,
        WorkingCopyStateProbe,
    };
    use work::sync::Dirtiness;

    use super::VcsWorkingCopyStatus;
    use crate::sync::fetch::WorkingCopyStatus as _;

    struct StubRepository {
        root: Option<&'static str>,
        state: Result<Vec<&'static str>, ()>,
    }

    impl WorkingCopyStateProbe for StubRepository {
        fn working_copy_state(
            &self,
            root: &Path,
            kind: VcsKind,
        ) -> Result<WorkingCopyState, kernel::Error> {
            assert_eq!((root, kind), (Path::new("/repo"), VcsKind::Git));
            self.state
                .clone()
                .map(|dirty| WorkingCopyState {
                    base_commits: vec!["abc123".to_owned()],
                    dirty_paths: dirty.into_iter().map(str::to_owned).collect(),
                })
                .map_err(|()| kernel::Error::Failed("unreadable".to_owned()))
        }
    }

    impl RepositoryProbe for StubRepository {
        fn facts_at(&self, _start: &Path) -> Option<RepoFacts> {
            self.root.map(|root| RepoFacts {
                root: PathBuf::from(root),
                name: "repo".to_owned(),
                kind: VcsKind::Git,
                revision: None,
            })
        }

        fn user_name_at(&self, _start: &Path) -> Option<String> {
            None
        }
    }

    fn dirtiness(repository: &StubRepository, path: &str) -> Dirtiness {
        VcsWorkingCopyStatus::probed_from(Path::new("/repo/meta"), repository)
            .is_dirty(Path::new(path))
    }

    #[test]
    fn a_changed_path_is_dirty_and_an_unchanged_one_clean() {
        let repository = StubRepository {
            root: Some("/repo"),
            state: Ok(vec!["meta/work/0001-a.md"]),
        };

        assert_eq!(
            dirtiness(&repository, "/repo/meta/work/0001-a.md"),
            Dirtiness::Dirty
        );
        assert_eq!(
            dirtiness(&repository, "/repo/meta/work/0002-b.md"),
            Dirtiness::Clean
        );
    }

    #[test]
    fn a_path_outside_the_working_copy_is_unknown() {
        let repository = StubRepository {
            root: Some("/repo"),
            state: Ok(vec![]),
        };

        assert_eq!(dirtiness(&repository, "/elsewhere.md"), Dirtiness::Unknown);
    }

    #[test]
    fn an_unreadable_working_copy_makes_every_path_unknown() {
        let repository = StubRepository {
            root: Some("/repo"),
            state: Err(()),
        };

        assert_eq!(
            dirtiness(&repository, "/repo/meta/work/0001-a.md"),
            Dirtiness::Unknown
        );
    }

    #[test]
    fn outside_a_repository_every_path_is_unknown() {
        let repository = StubRepository {
            root: None,
            state: Ok(vec![]),
        };

        assert_eq!(
            dirtiness(&repository, "/repo/meta/work/0001-a.md"),
            Dirtiness::Unknown
        );
    }
}
