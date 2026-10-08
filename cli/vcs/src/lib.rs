//! What a repository reports about itself, and the ports an adapter satisfies
//! to find out. The probing itself — filesystem walks, subprocesses — lives in
//! `vcs-adapters`; this crate only composes the facts.

pub mod checkout;
pub mod classify;
pub mod guard;
pub mod log;
pub mod mode;
pub mod origin_remote;
pub mod status;
pub mod tracking;

use std::path::Path;
use std::path::PathBuf;

/// The command set a repository's idiom calls for.
///
/// This is deliberately not a topology: a colocated checkout carries both
/// markers, and `Jj` wins there because git's index lags the jj working-copy
/// commit, so a git-shaped probe would read live edits as clean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsKind {
    Jj,
    Git,
    None,
}

impl VcsKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jj => "jj",
            Self::Git => "git",
            Self::None => "none",
        }
    }
}

/// The repository facts the corpus surfaces stamp artifacts with.
///
/// `root` is the working-copy root (a jj secondary workspace roots at its own
/// marker); `name` is the *repository* the working copy belongs to, so a
/// workspace stamps artifacts with the repository's name rather than the
/// ephemeral workspace directory's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoFacts {
    pub root: PathBuf,
    pub name: String,
    pub kind: VcsKind,
    pub revision: Option<String>,
}

/// Locates the repository a path belongs to.
pub trait RepoRoot {
    /// The working-copy root containing `start`, or `None` when there is no
    /// repository above it.
    fn discover(&self, start: &Path) -> Option<PathBuf>;

    /// The repository a working-copy root belongs to. A jj secondary workspace
    /// roots at its own working copy but shares the repository's store; by
    /// default the working-copy root is itself the repository root.
    fn repository_root(&self, working_copy_root: &Path) -> PathBuf {
        working_copy_root.to_path_buf()
    }
}

/// Reports a repository's idiom and its working-copy revision.
pub trait VcsProbe {
    fn kind(&self, root: &Path) -> VcsKind;

    /// The full working-copy revision, or `None` when the repository has none
    /// and when the probe cannot answer. A caller cannot distinguish the two;
    /// an adapter is expected to log the failure.
    ///
    /// A sha256-format repository is unsupported: the underlying `gix` query
    /// fails to read one at all, so this folds to `None` like any other probe
    /// failure, rather than misreading the revision.
    fn revision(&self, root: &Path, kind: VcsKind) -> Option<String>;
}

/// Renders a repository's working-copy status and recent history into the
/// backend-neutral model the renderer consumes.
///
/// Unlike the sibling `Option`-returning ports, this one keeps the `Err` arm:
/// the `vcs-cli` boundary folds it to the never-fail `(status|log unavailable)`
/// fallback and, on the `ACCELERATOR_LOG` path, names the failing adapter.
/// `kernel::Error` is the sanctioned port error — `vcs` depends on `kernel`, not
/// on `gix`/`jj-lib` — so an adapter propagates its own error through the shared
/// `From` conversion rather than introducing a public error type here.
pub trait VcsReporter {
    /// # Errors
    ///
    /// When `root` carries the named idiom but its status cannot be read.
    fn status_report(
        &self,
        root: &Path,
        kind: VcsKind,
    ) -> Result<status::StatusReport, kernel::Error>;

    /// # Errors
    ///
    /// When `root` carries the named idiom but its history cannot be read.
    fn log_report(
        &self,
        root: &Path,
        kind: VcsKind,
    ) -> Result<log::LogReport, kernel::Error>;
}

/// Reports a repository's configured user identity.
pub trait UserIdentityProbe {
    /// The configured `user.name` for the repository at `root`, given its
    /// `kind`, or `None` when unconfigured, absent, or unanswerable. A caller
    /// cannot distinguish the three; an adapter is expected to log the
    /// failure.
    fn user_name(&self, root: &Path, kind: VcsKind) -> Option<String>;
}

/// The commits a working copy is based on and the paths that differ from
/// them, taken from a single read of the repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkingCopyState {
    pub base_commits: Vec<String>,
    pub dirty_paths: Vec<String>,
}

/// Reads a working copy's base commits and dirty paths together.
pub trait WorkingCopyStateProbe {
    /// The commits the working copy at `root` is based on, and the
    /// repo-relative paths that differ from them, read in one pass so the two
    /// cannot disagree. Untracked files count as dirty and ignored files do
    /// not, in either idiom: an untracked file is the least recoverable thing
    /// in a working copy, so a caller gating a destructive write must see it.
    ///
    /// # Errors
    ///
    /// When `root` carries the named idiom but its status or diff cannot be
    /// computed.
    fn working_copy_state(
        &self,
        root: &Path,
        kind: VcsKind,
    ) -> Result<WorkingCopyState, kernel::Error>;
}

/// The repository questions a consumer asks from wherever it starts, rather
/// than from a root it has already discovered.
pub trait RepositoryProbe: WorkingCopyStateProbe {
    /// See [`facts`].
    fn facts_at(&self, start: &Path) -> Option<RepoFacts>;

    /// See [`user_name`].
    fn user_name_at(&self, start: &Path) -> Option<String>;
}

impl<T> RepositoryProbe for T
where
    T: RepoRoot + VcsProbe + UserIdentityProbe + WorkingCopyStateProbe,
{
    fn facts_at(&self, start: &Path) -> Option<RepoFacts> {
        facts(start, self, self)
    }

    fn user_name_at(&self, start: &Path) -> Option<String> {
        user_name(start, self, self, self)
    }
}

/// The configured VCS user name for the repository containing `start`.
///
/// `None` outside a repository and when the probe cannot answer.
#[must_use]
pub fn user_name(
    start: &Path,
    root: &dyn RepoRoot,
    probe: &dyn VcsProbe,
    identity: &dyn UserIdentityProbe,
) -> Option<String> {
    let root_path = root.discover(start)?;
    let kind = probe.kind(&root_path);
    identity.user_name(&root_path, kind)
}

/// The facts for the repository containing `start`.
///
/// `None` when no repository contains `start`, so a marker-less tree is
/// representable rather than fabricated as a blank root and name.
#[must_use]
pub fn facts(
    start: &Path,
    root: &dyn RepoRoot,
    probe: &dyn VcsProbe,
) -> Option<RepoFacts> {
    let root_path = root.discover(start)?;
    let repository_root = root.repository_root(&root_path);
    let name = repository_root.file_name()?.to_str()?.to_owned();
    let kind = probe.kind(&root_path);
    let revision = probe.revision(&root_path, kind);

    Some(RepoFacts {
        root: root_path,
        name,
        kind,
        revision,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        facts, user_name, RepoFacts, RepoRoot, RepositoryProbe,
        UserIdentityProbe, VcsKind, VcsProbe, WorkingCopyState,
        WorkingCopyStateProbe,
    };

    struct FixedRoot(Option<PathBuf>);

    impl RepoRoot for FixedRoot {
        fn discover(&self, _start: &Path) -> Option<PathBuf> {
            self.0.clone()
        }
    }

    struct FixedProbe {
        kind: VcsKind,
        revision: Option<String>,
    }

    impl VcsProbe for FixedProbe {
        fn kind(&self, _root: &Path) -> VcsKind {
            self.kind
        }

        fn revision(&self, _root: &Path, _kind: VcsKind) -> Option<String> {
            self.revision.clone()
        }
    }

    fn probe(kind: VcsKind, revision: Option<&str>) -> FixedProbe {
        FixedProbe {
            kind,
            revision: revision.map(str::to_owned),
        }
    }

    #[test]
    fn composes_the_facts_of_the_discovered_repository() {
        let root = FixedRoot(Some(PathBuf::from("/tmp/some-repo")));

        assert_eq!(
            facts(
                Path::new("/tmp/some-repo/meta/work"),
                &root,
                &probe(VcsKind::Jj, Some("abc123"))
            ),
            Some(RepoFacts {
                root: PathBuf::from("/tmp/some-repo"),
                name: "some-repo".to_owned(),
                kind: VcsKind::Jj,
                revision: Some("abc123".to_owned()),
            })
        );
    }

    #[test]
    fn the_name_is_the_final_component_of_the_root() {
        let root = FixedRoot(Some(PathBuf::from("/a/b/c/deeply-nested")));
        let derived = facts(
            Path::new("/a/b/c/deeply-nested"),
            &root,
            &probe(VcsKind::Git, None),
        );

        assert_eq!(
            derived.map(|facts| facts.name).as_deref(),
            Some("deeply-nested")
        );
    }

    #[test]
    fn a_tree_with_no_repository_has_no_facts() {
        let derived = facts(
            Path::new("/tmp/loose"),
            &FixedRoot(None),
            &probe(VcsKind::None, None),
        );

        assert_eq!(derived, None);
    }

    #[test]
    fn an_unanswerable_revision_leaves_the_rest_of_the_facts_intact() {
        let root = FixedRoot(Some(PathBuf::from("/tmp/no-commits")));
        let derived = facts(
            Path::new("/tmp/no-commits"),
            &root,
            &probe(VcsKind::Git, None),
        );

        assert_eq!(
            derived,
            Some(RepoFacts {
                root: PathBuf::from("/tmp/no-commits"),
                name: "no-commits".to_owned(),
                kind: VcsKind::Git,
                revision: None,
            })
        );
    }

    struct FixedIdentity(Option<&'static str>);

    impl UserIdentityProbe for FixedIdentity {
        fn user_name(&self, _root: &Path, _kind: VcsKind) -> Option<String> {
            self.0.map(str::to_owned)
        }
    }

    #[test]
    fn composes_the_user_name_of_the_discovered_repository() {
        let root = FixedRoot(Some(PathBuf::from("/tmp/some-repo")));

        assert_eq!(
            user_name(
                Path::new("/tmp/some-repo/meta/work"),
                &root,
                &probe(VcsKind::Jj, None),
                &FixedIdentity(Some("Toby Clemson")),
            ),
            Some("Toby Clemson".to_owned())
        );
    }

    #[test]
    fn a_tree_with_no_repository_has_no_user_name() {
        let derived = user_name(
            Path::new("/tmp/loose"),
            &FixedRoot(None),
            &probe(VcsKind::None, None),
            &FixedIdentity(Some("Toby Clemson")),
        );

        assert_eq!(derived, None);
    }

    #[test]
    fn an_unanswerable_identity_is_none() {
        let root = FixedRoot(Some(PathBuf::from("/tmp/unconfigured")));
        let derived = user_name(
            Path::new("/tmp/unconfigured"),
            &root,
            &probe(VcsKind::Git, None),
            &FixedIdentity(None),
        );

        assert_eq!(derived, None);
    }

    struct StubRepository {
        root: Option<PathBuf>,
        user_name: Option<&'static str>,
    }

    impl RepoRoot for StubRepository {
        fn discover(&self, _start: &Path) -> Option<PathBuf> {
            self.root.clone()
        }
    }

    impl VcsProbe for StubRepository {
        fn kind(&self, _root: &Path) -> VcsKind {
            VcsKind::Git
        }

        fn revision(&self, _root: &Path, _kind: VcsKind) -> Option<String> {
            Some("abc123".to_owned())
        }
    }

    impl UserIdentityProbe for StubRepository {
        fn user_name(&self, _root: &Path, _kind: VcsKind) -> Option<String> {
            self.user_name.map(str::to_owned)
        }
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

    fn repository_at(root: &str) -> StubRepository {
        StubRepository {
            root: Some(PathBuf::from(root)),
            user_name: Some("Toby Clemson"),
        }
    }

    #[test]
    fn a_repository_probe_composes_the_facts_of_the_discovered_repository() {
        let probe: &dyn RepositoryProbe = &repository_at("/tmp/some-repo");

        assert_eq!(
            probe.facts_at(Path::new("/tmp/some-repo/meta/work")),
            Some(RepoFacts {
                root: PathBuf::from("/tmp/some-repo"),
                name: "some-repo".to_owned(),
                kind: VcsKind::Git,
                revision: Some("abc123".to_owned()),
            })
        );
    }

    #[test]
    fn a_repository_probe_composes_the_user_name_of_the_discovered_repository()
    {
        let probe: &dyn RepositoryProbe = &repository_at("/tmp/some-repo");

        assert_eq!(
            probe.user_name_at(Path::new("/tmp/some-repo/meta/work")),
            Some("Toby Clemson".to_owned())
        );
    }

    #[test]
    fn a_repository_probe_outside_a_repository_has_no_facts_or_user_name() {
        let probe: &dyn RepositoryProbe = &StubRepository {
            root: None,
            user_name: Some("Toby Clemson"),
        };

        assert_eq!(probe.facts_at(Path::new("/tmp/loose")), None);
        assert_eq!(probe.user_name_at(Path::new("/tmp/loose")), None);
    }
}
