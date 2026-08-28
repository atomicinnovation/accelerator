//! The command-set mode a checkout calls for: `jj`, `jj-colocated`, or `git`.
//!
//! Resolved from the two library-backed queries rather than a `[ -d ... ]`
//! test on the combined walk's own root — which makes this robust to a `.git`
//! *file* (a worktree/submodule marker) that `-d` cannot see.

use std::path::Path;
use std::path::PathBuf;

use crate::checkout::DualRoots;

/// The command set a checkout's markers call for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Jj,
    JjColocated,
    Git,
}

/// The two `InProcessProbe` queries [`determine`] needs.
pub trait ModeProbe {
    /// The jj workspace root containing `start`.
    ///
    /// # Errors
    ///
    /// When a `.jj` directory is present but the workspace cannot be loaded.
    fn jj_workspace_root(
        &self,
        start: &Path,
    ) -> Result<Option<PathBuf>, kernel::Error>;

    /// Never itself an `Err` — see [`DualRoots`].
    fn dual_roots(&self, start: &Path) -> DualRoots;
}

/// Determines the mode of the checkout containing `start`.
///
/// `jj` wins if a jj workspace is present at all; it is `jj-colocated` only
/// when the git working-copy root the same `start` resolves to (an
/// independent, unbounded walk) is the *same directory* as the jj workspace
/// root — the single-directory-carries-both-markers case, not merely "a git
/// repository exists somewhere in the ancestry" (which a jj secondary
/// workspace nested inside an unrelated git checkout would also satisfy).
///
/// # Errors
///
/// When the jj workspace is present but cannot be loaded.
pub fn determine(
    start: &Path,
    probe: &dyn ModeProbe,
) -> Result<Mode, kernel::Error> {
    let Some(jj_root) = probe.jj_workspace_root(start)? else {
        return Ok(Mode::Git);
    };
    let git_root = probe.dual_roots(start).git.ok().flatten();
    if git_root.as_deref() == Some(jj_root.as_path()) {
        Ok(Mode::JjColocated)
    } else {
        Ok(Mode::Jj)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::determine;
    use super::Mode;
    use super::ModeProbe;
    use crate::checkout::DualRoots;

    type TestError = Box<dyn std::error::Error>;

    #[derive(Default, Clone)]
    struct StubProbe {
        jj_workspace_root: Option<Result<Option<PathBuf>, String>>,
        dual_git: Option<Result<Option<PathBuf>, String>>,
    }

    fn to_kernel<T>(result: Result<T, String>) -> Result<T, kernel::Error> {
        result.map_err(kernel::Error::Failed)
    }

    impl ModeProbe for StubProbe {
        fn jj_workspace_root(
            &self,
            _start: &Path,
        ) -> Result<Option<PathBuf>, kernel::Error> {
            to_kernel(self.jj_workspace_root.clone().unwrap_or(Ok(None)))
        }

        fn dual_roots(&self, _start: &Path) -> DualRoots {
            DualRoots {
                git: to_kernel(self.dual_git.clone().unwrap_or(Ok(None))),
                jj: Ok(None),
            }
        }
    }

    fn start() -> PathBuf {
        PathBuf::from("/tmp/somewhere")
    }

    fn determined(probe: &StubProbe) -> Result<Mode, TestError> {
        determine(&start(), probe).map_err(|error| error.to_string().into())
    }

    #[test]
    fn no_jj_workspace_is_git_mode() -> Result<(), TestError> {
        let probe = StubProbe {
            jj_workspace_root: Some(Ok(None)),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
        };
        assert_eq!(determined(&probe)?, Mode::Git);
        Ok(())
    }

    #[test]
    fn a_jj_workspace_with_no_git_at_the_same_root_is_pure_jj(
    ) -> Result<(), TestError> {
        let probe = StubProbe {
            jj_workspace_root: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_git: Some(Ok(None)),
        };
        assert_eq!(determined(&probe)?, Mode::Jj);
        Ok(())
    }

    #[test]
    fn a_jj_workspace_with_git_at_a_different_root_is_pure_jj_not_colocated(
    ) -> Result<(), TestError> {
        // A jj secondary workspace nested inside an unrelated git checkout:
        // git is present somewhere in the ancestry, but not at the jj
        // workspace's own root, so this must not read as colocated.
        let probe = StubProbe {
            jj_workspace_root: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/outer-git")))),
        };
        assert_eq!(determined(&probe)?, Mode::Jj);
        Ok(())
    }

    #[test]
    fn a_jj_workspace_with_git_at_the_same_root_is_colocated(
    ) -> Result<(), TestError> {
        let probe = StubProbe {
            jj_workspace_root: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
        };
        assert_eq!(determined(&probe)?, Mode::JjColocated);
        Ok(())
    }

    #[test]
    fn an_unreadable_git_side_degrades_to_pure_jj_not_an_error(
    ) -> Result<(), TestError> {
        // dual_roots' git side is a comparison, not a fact lookup: an `Err`
        // there degrades to "not colocated" rather than propagating.
        let probe = StubProbe {
            jj_workspace_root: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_git: Some(Err("could not open the repository".to_owned())),
        };
        assert_eq!(determined(&probe)?, Mode::Jj);
        Ok(())
    }

    #[test]
    fn jj_workspace_root_failing_propagates_rather_than_guessing_a_mode() {
        let probe = StubProbe {
            jj_workspace_root: Some(Err(
                "could not load the jj workspace".to_owned()
            )),
            dual_git: Some(Ok(None)),
        };
        assert!(determine(&start(), &probe).is_err());
    }
}
