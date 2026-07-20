//! What a repository reports about itself, and the ports an adapter satisfies
//! to find out. The probing itself — filesystem walks, subprocesses — lives in
//! `vcs-adapters`; this crate only composes the facts.

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
    fn revision(&self, root: &Path, kind: VcsKind) -> Option<String>;
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

    use super::{facts, RepoFacts, RepoRoot, VcsKind, VcsProbe};

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
}
