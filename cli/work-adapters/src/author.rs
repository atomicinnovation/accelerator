//! Resolves the current VCS identity for `work create`'s `author`
//! fallback, when `--author` is not given.
//!
//! [`VcsBackedIdentityProbe`] is the concrete implementation of
//! `work::create::VcsIdentityProbe` — the port `work`'s own
//! `resolve_author` is written against, so its decision logic (explicit
//! flag wins, else the probe, else an error) stays pure and testable with
//! a double there, and this crate's dependency on `vcs`/`vcs-adapters`
//! stays an adapter-to-adapter implementation detail `work` and `work-cli`
//! never need to know about.
//!
//! Delegates to `vcs::user_name`/`vcs_adapters::library::InProcessProbe`
//! wholesale — root discovery, kind detection, and the git/jj config read
//! all happen in-process there, so this module carries no VCS-specific logic
//! of its own. `InProcessProbe`'s own test suite, including
//! `vcs-adapters/tests/user_name.rs`, covers that behaviour.

use std::path::Path;

use vcs_adapters::library::InProcessProbe;
use work::create::VcsIdentityProbe;

/// The configured VCS user name for the repository containing `start`.
///
/// jj's or git's `user.name` (jj-colocated wins), or `None` on any failure —
/// unconfigured, no repository, or the probe could not answer.
#[must_use]
pub fn vcs_user_at(start: &Path) -> Option<String> {
    let probe = InProcessProbe;
    vcs::user_name(start, &probe, &probe, &probe)
}

/// The configured VCS user name for the repository containing the process
/// working directory. See [`vcs_user_at`].
#[must_use]
pub fn current_vcs_user() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    vcs_user_at(&cwd)
}

/// The real, `vcs`/`vcs-adapters`-backed implementation of
/// `work::create::VcsIdentityProbe`, reading the process working directory's
/// repository.
#[derive(Debug, Clone, Copy, Default)]
pub struct VcsBackedIdentityProbe;

impl VcsIdentityProbe for VcsBackedIdentityProbe {
    fn current_user_name(&self) -> Option<String> {
        current_vcs_user()
    }
}

/// A [`VcsIdentityProbe`] that reads a named repository's identity.
///
/// An operation that already knows the repository it acts on resolves its
/// author from that repository rather than the process working directory —
/// where the process runs is not necessarily the repository being synced.
#[derive(Debug, Clone, Copy)]
pub struct RepositoryIdentityProbe<'a> {
    root: &'a Path,
}

impl<'a> RepositoryIdentityProbe<'a> {
    #[must_use]
    pub const fn new(root: &'a Path) -> Self {
        Self { root }
    }
}

impl VcsIdentityProbe for RepositoryIdentityProbe<'_> {
    fn current_user_name(&self) -> Option<String> {
        vcs_user_at(self.root)
    }
}
