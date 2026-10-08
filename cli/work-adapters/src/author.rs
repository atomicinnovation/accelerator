//! Resolves the current VCS identity for `work create`'s `author`
//! fallback, when `--author` is not given.
//!
//! [`VcsBackedIdentityProbe`] is the concrete implementation of
//! `work::create::VcsIdentityProbe` — the port `work`'s own
//! `resolve_author` is written against, so its decision logic (explicit
//! flag wins, else the probe, else an error) stays pure and testable with
//! a double there.
//!
//! Root discovery, kind detection and the git/jj config read all happen
//! behind the injected `vcs::RepositoryProbe`, so this module carries no
//! VCS-specific logic of its own.

use std::path::Path;

use vcs::RepositoryProbe;
use work::create::VcsIdentityProbe;

/// The configured VCS user name for the repository containing `start`.
///
/// jj's or git's `user.name` (jj-colocated wins), or `None` on any failure —
/// unconfigured, no repository, or the probe could not answer.
#[must_use]
pub fn vcs_user_at(
    start: &Path,
    repository: &dyn RepositoryProbe,
) -> Option<String> {
    repository.user_name_at(start)
}

/// The configured VCS user name for the repository containing the process
/// working directory. See [`vcs_user_at`].
#[must_use]
pub fn current_vcs_user(repository: &dyn RepositoryProbe) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    vcs_user_at(&cwd, repository)
}

/// A [`VcsIdentityProbe`] reading the process working directory's
/// repository.
#[derive(Clone, Copy)]
pub struct VcsBackedIdentityProbe<'a> {
    repository: &'a dyn RepositoryProbe,
}

impl<'a> VcsBackedIdentityProbe<'a> {
    #[must_use]
    pub const fn new(repository: &'a dyn RepositoryProbe) -> Self {
        Self { repository }
    }
}

impl VcsIdentityProbe for VcsBackedIdentityProbe<'_> {
    fn current_user_name(&self) -> Option<String> {
        current_vcs_user(self.repository)
    }
}

/// A [`VcsIdentityProbe`] that reads a named repository's identity.
///
/// An operation that already knows the repository it acts on resolves its
/// author from that repository rather than the process working directory —
/// where the process runs is not necessarily the repository being synced.
#[derive(Clone, Copy)]
pub struct RepositoryIdentityProbe<'a> {
    root: &'a Path,
    repository: &'a dyn RepositoryProbe,
}

impl<'a> RepositoryIdentityProbe<'a> {
    #[must_use]
    pub const fn new(
        root: &'a Path,
        repository: &'a dyn RepositoryProbe,
    ) -> Self {
        Self { root, repository }
    }
}

impl VcsIdentityProbe for RepositoryIdentityProbe<'_> {
    fn current_user_name(&self) -> Option<String> {
        vcs_user_at(self.root, self.repository)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use vcs::{
        RepoFacts, RepositoryProbe, VcsKind, WorkingCopyState,
        WorkingCopyStateProbe,
    };
    use work::create::VcsIdentityProbe;

    use super::{vcs_user_at, RepositoryIdentityProbe, VcsBackedIdentityProbe};

    #[derive(Default)]
    struct StubRepository {
        asked_from: RefCell<Vec<PathBuf>>,
    }

    impl WorkingCopyStateProbe for StubRepository {
        fn working_copy_state(
            &self,
            _root: &Path,
            _kind: VcsKind,
        ) -> Result<WorkingCopyState, kernel::Error> {
            Ok(WorkingCopyState::default())
        }
    }

    impl RepositoryProbe for StubRepository {
        fn facts_at(&self, _start: &Path) -> Option<RepoFacts> {
            None
        }

        fn user_name_at(&self, start: &Path) -> Option<String> {
            self.asked_from.borrow_mut().push(start.to_path_buf());
            (start != Path::new("/tmp/loose")).then(|| "Toby".to_owned())
        }
    }

    #[test]
    fn the_user_name_is_the_repository_s_own() {
        let repository = StubRepository::default();

        assert_eq!(
            vcs_user_at(Path::new("/tmp/repo"), &repository),
            Some("Toby".to_owned())
        );
        assert_eq!(vcs_user_at(Path::new("/tmp/loose"), &repository), None);
    }

    #[test]
    fn a_repository_identity_probe_asks_from_its_named_root() {
        let repository = StubRepository::default();
        let probe =
            RepositoryIdentityProbe::new(Path::new("/tmp/repo"), &repository);

        assert_eq!(probe.current_user_name(), Some("Toby".to_owned()));
        assert_eq!(*repository.asked_from.borrow(), [Path::new("/tmp/repo")]);
    }

    #[test]
    fn a_vcs_backed_identity_probe_asks_from_the_working_directory(
    ) -> Result<(), std::io::Error> {
        let repository = StubRepository::default();
        let probe = VcsBackedIdentityProbe::new(&repository);

        assert_eq!(probe.current_user_name(), Some("Toby".to_owned()));
        assert_eq!(*repository.asked_from.borrow(), [std::env::current_dir()?]);
        Ok(())
    }
}
